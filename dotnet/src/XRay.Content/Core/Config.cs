using System.Text.Json;
using System.Text.Json.Serialization;
using XRay.Content.Types;

namespace XRay.Content.Core;

/// <summary>
/// Output format. Known variants serialize as lowercase strings ("plain", "markdown", "djot",
/// "html", "json", "structured"); <see cref="Custom"/> serializes as its bare renderer name.
/// Default = Plain. Mirrors Rust `OutputFormat` (FromStr / Display).
/// </summary>
[JsonConverter(typeof(OutputFormatConverter))]
public readonly struct OutputFormat : IEquatable<OutputFormat>
{
    public enum Kind { Plain, Markdown, Djot, Html, Json, Structured, DocTags, Custom }

    public Kind Which { get; }
    public string? CustomName { get; }

    private OutputFormat(Kind which, string? customName = null)
    {
        Which = which;
        CustomName = customName;
    }

    public static readonly OutputFormat Plain = new(Kind.Plain);
    public static readonly OutputFormat Markdown = new(Kind.Markdown);
    public static readonly OutputFormat Djot = new(Kind.Djot);
    public static readonly OutputFormat Html = new(Kind.Html);
    public static readonly OutputFormat Json = new(Kind.Json);
    public static readonly OutputFormat Structured = new(Kind.Structured);
    public static readonly OutputFormat DocTags = new(Kind.DocTags);
    public static OutputFormat Custom(string name) => new(Kind.Custom, name);

    /// <summary>Parse from a string (never fails; unknown → Custom of the lowercased string).</summary>
    public static OutputFormat FromString(string s)
    {
        string lower = s.ToLowerInvariant();
        return lower switch
        {
            "plain" or "text" => Plain,
            "markdown" or "md" => Markdown,
            "djot" => Djot,
            "html" => Html,
            "json" => Json,
            "structured" or "structured-ocr" => Structured,
            "doctags" => DocTags,
            _ => Custom(lower),
        };
    }

    public override string ToString() => Which switch
    {
        Kind.Plain => "plain",
        Kind.Markdown => "markdown",
        Kind.Djot => "djot",
        Kind.Html => "html",
        Kind.Json => "json",
        Kind.Structured => "structured",
        Kind.DocTags => "doctags",
        Kind.Custom => CustomName ?? "",
        _ => "plain",
    };

    public bool Equals(OutputFormat other) => Which == other.Which && CustomName == other.CustomName;
    public override bool Equals(object? obj) => obj is OutputFormat o && Equals(o);
    public override int GetHashCode() => HashCode.Combine(Which, CustomName);
    public static bool operator ==(OutputFormat a, OutputFormat b) => a.Equals(b);
    public static bool operator !=(OutputFormat a, OutputFormat b) => !a.Equals(b);
}

public sealed class OutputFormatConverter : JsonConverter<OutputFormat>
{
    public override OutputFormat Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) =>
        OutputFormat.FromString(reader.GetString() ?? "plain");

    public override void Write(Utf8JsonWriter writer, OutputFormat value, JsonSerializerOptions options) =>
        writer.WriteStringValue(value.ToString());
}

/// <summary>Trimmed, native-only extraction configuration.</summary>
public sealed class ExtractionConfig
{
    public ResultFormat ResultFormat { get; set; } = ResultFormat.Unified;
    public OutputFormat OutputFormat { get; set; } = OutputFormat.Plain;
    public bool IncludeDocumentStructure { get; set; }

    /// <summary>
    /// Also render each page in <see cref="OutputFormat"/>, into
    /// <see cref="Types.PageContent.FormattedContent"/>.
    /// </summary>
    /// <remarks>
    /// Off by default, and not part of the wire format: a page's <see cref="Types.PageContent.Content"/>
    /// is the concatenation of its element texts, which is what upstream produces and what the
    /// goldens pin. Turning this on costs one extra render per page and is what a caller needs to
    /// have the document as markup *and* know which page each part came from.
    /// </remarks>
    [JsonIgnore]
    public bool RenderPagesInOutputFormat { get; set; }

    /// <summary>
    /// Keep the bytes of the images an extractor finds, in <c>ExtractedDocument.Images</c>.
    /// </summary>
    /// <remarks>
    /// Off by default, which is upstream's default too: its <c>images</c> is an
    /// <c>Option&lt;ImageExtractionConfig&gt;</c> whose <c>None</c> means no image extraction, and
    /// <c>needs_image_data()</c> reads through it. Carrying a document's images costs their whole
    /// encoded size in memory and in any serialized result, so it is asked for rather than
    /// assumed — see <see cref="NeedsImageData"/> for what else turns it on.
    /// <para>
    /// It currently gates <see cref="XRay.Content.Extractors.ImageExtractor"/> only. The
    /// extractors that already attach images unconditionally — docx, rtf, epub, hwp, odf — are
    /// untouched: upstream gates those on the same flag, but retrofitting it here would *remove*
    /// images that callers of this package already receive.
    /// </para>
    /// </remarks>
    public bool ExtractImages { get; set; }

    // Content-relevant option stub (defaults; extractors read this later).
    public bool ExtractTables { get; set; } = true;

    /// <summary>
    /// Decode the QR codes inside every extracted image, writing them to
    /// <c>ExtractedImage.QrCodes</c> and appending their payloads to the document text.
    /// </summary>
    /// <remarks>
    /// Opt-in, as upstream has it: decoding runs the detector over every image in the document,
    /// which is real work to do on a caller's behalf without being asked.
    /// </remarks>
    [JsonPropertyName("qr_codes")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public bool? QrCodes { get; set; }

    /// <summary>
    /// The optional OCR pass. Null or <see cref="OcrMode.Disabled"/> — the default — means no
    /// OCR, which is the port's behaviour before OCR existed.
    /// </summary>
    /// <remarks>
    /// This is not upstream's OCR configuration. OCR is a deviation for this port, on a
    /// different engine; see "Deviation: optional OCR" in <c>CLAUDE.md</c>.
    /// </remarks>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public OcrOptions? Ocr { get; set; }

    /// <summary>
    /// PDF-specific settings: passwords for encrypted documents, annotation and embedded-file
    /// extraction. Null takes the defaults of <see cref="PdfOptions"/>.
    /// </summary>
    [JsonPropertyName("pdf")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public PdfOptions? Pdf { get; set; }

    /// <summary>
    /// Whether anything in this configuration needs an extracted image's bytes, rather than just
    /// the fact that an image was there.
    /// </summary>
    /// <remarks>
    /// Upstream's <c>needs_image_data()</c>, which is <c>extract_images || captioning || qr_codes</c>;
    /// captioning is an LLM feature this port excludes, so it has no term here.
    /// <para>
    /// The OCR term is the port's own. Upstream recognises a standalone image *inside* its image
    /// extractor, so its OCR branch never needs the bytes to travel; here OCR is a separate pass
    /// over the finished document (see "Deviation: optional OCR" in <c>CLAUDE.md</c>), and
    /// <see cref="OcrMode.AllImages"/> finds its work by walking <c>InternalDocument.Images</c>.
    /// Without this term that pass has nothing to walk for a standalone image and silently
    /// recognises nothing. <see cref="OcrMode.ScanOnly"/> is not included: it rasterises PDF pages
    /// from the original bytes and never reads the image collection.
    /// </para>
    /// </remarks>
    public bool NeedsImageData() =>
        ExtractImages
     || QrCodes == true
     || Ocr?.Mode == OcrMode.AllImages;

    /// <summary>
    /// Limits applied to hostile input. <c>null</c> takes <see cref="SecurityLimits"/>' defaults,
    /// which is what upstream's <c>Option&lt;SecurityLimits&gt;</c> does with <c>None</c>.
    /// </summary>
    [JsonPropertyName("security_limits")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public SecurityLimits? SecurityLimits { get; set; }

    /// <summary>
    /// How <c>OutputFormat.Html</c> is rendered. <c>null</c> keeps the markdown-based renderer;
    /// setting it selects <see cref="XRay.Content.Rendering.StyledHtmlRenderer"/>.
    /// </summary>
    [JsonPropertyName("html_output")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public HtmlOutputConfig? HtmlOutput { get; set; }

    /// <summary>
    /// The input's file name, when the caller knows it. Used to fall back to extension-based
    /// language detection where content-based detection — a shebang — says nothing.
    /// </summary>
    /// <remarks>
    /// Not part of the wire format, matching upstream's <c>#[serde(skip)]</c>: it describes the
    /// input rather than configuring the extraction.
    /// </remarks>
    [JsonIgnore]
    public string? SourceName { get; set; }

    /// <summary>
    /// Port-local behavioural knobs (deadlines, implementation switches). Defaults to
    /// <see cref="XRayOptions.Default"/>. Not part of the wire format: everything above mirrors
    /// upstream's config field for field, and these have no upstream counterpart.
    /// </summary>
    [JsonIgnore]
    public XRayOptions Options { get; set; } = XRayOptions.Default;
}

/// <summary>Kind of extraction input. Serialized as bare snake_case string.</summary>
public enum ExtractInputKind
{
    Bytes,
    Uri,
}

/// <summary>An extraction input: raw bytes with a MIME type, or a URI/path.</summary>
public sealed class ExtractInput
{
    public ExtractInputKind Kind { get; set; } = ExtractInputKind.Uri;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public byte[]? Bytes { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Uri { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? MimeType { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Filename { get; set; }

    public static ExtractInput FromBytes(byte[] bytes, string mimeType, string? filename = null) => new()
    {
        Kind = ExtractInputKind.Bytes,
        Bytes = bytes,
        MimeType = mimeType,
        Filename = filename,
    };

    public static ExtractInput FromUri(string uri) => new()
    {
        Kind = ExtractInputKind.Uri,
        Uri = uri,
    };
}

public sealed class ExtractionErrorItem
{
    public long Index { get; set; }
    public uint Code { get; set; }
    public string ErrorType { get; set; } = "";
    public string Source { get; set; } = "";
    public string Message { get; set; } = "";
}

public sealed class ExtractionSummary
{
    public long Inputs { get; set; }
    public long Results { get; set; }
    public long Errors { get; set; }
}

/// <summary>Batch envelope of extraction results plus per-input errors.</summary>
public sealed class ExtractionResult
{
    public List<ExtractedDocument> Results { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]
    public List<ExtractionErrorItem> Errors { get; set; } = new();

    public ExtractionSummary Summary { get; set; } = new();
}
