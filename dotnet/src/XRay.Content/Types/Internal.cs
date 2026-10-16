using System.Buffers.Binary;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using XRay.Content.Internal.Blake3;

namespace XRay.Content.Types;

/// <summary>
/// Deterministic element identifier: "ie-" + 12 lowercase hex chars (first 6 bytes of a
/// BLAKE3 hash of discriminant ++ text ++ page(LE u32, u32::MAX when None) ++ index(LE u32)).
/// Serializes as a plain string.
/// </summary>
[JsonConverter(typeof(InternalElementIdConverter))]
public readonly struct InternalElementId : IEquatable<InternalElementId>
{
    private readonly string _value;

    private InternalElementId(string value) => _value = value;

    public static InternalElementId Generate(string kindDiscriminant, string text, uint? page, uint index)
    {
        var hasher = new Blake3Hasher();
        hasher.Update(Encoding.UTF8.GetBytes(kindDiscriminant));
        hasher.Update(Encoding.UTF8.GetBytes(text));
        Span<byte> le = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(le, page ?? uint.MaxValue);
        hasher.Update(le);
        BinaryPrimitives.WriteUInt32LittleEndian(le, index);
        hasher.Update(le);

        Span<byte> hash = stackalloc byte[32];
        hasher.Finalize(hash);
        var sb = new StringBuilder(15);
        sb.Append("ie-");
        for (int i = 0; i < 6; i++)
            sb.Append(hash[i].ToString("x2"));
        return new InternalElementId(sb.ToString());
    }

    public static InternalElementId FromString(string id) => new(id);

    public string AsString() => _value ?? "";

    public override string ToString() => AsString();

    public bool Equals(InternalElementId other) => AsString() == other.AsString();

    public override bool Equals(object? obj) => obj is InternalElementId o && Equals(o);

    public override int GetHashCode() => AsString().GetHashCode();
}

public sealed class InternalElementIdConverter : JsonConverter<InternalElementId>
{
    public override InternalElementId Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) =>
        InternalElementId.FromString(reader.GetString() ?? "");

    public override void Write(Utf8JsonWriter writer, InternalElementId value, JsonSerializerOptions options) =>
        writer.WriteStringValue(value.AsString());
}

/// <summary>Target of a relationship — a resolved index or an unresolved key.</summary>
[JsonConverter(typeof(RelationshipTargetConverter))]
public sealed class RelationshipTarget
{
    public uint? Index { get; }
    public string? Key { get; }

    private RelationshipTarget(uint? index, string? key)
    {
        Index = index;
        Key = key;
    }

    public bool IsIndex => Index is not null;

    public static RelationshipTarget FromIndex(uint index) => new(index, null);
    public static RelationshipTarget FromKey(string key) => new(null, key);
}

/// <summary>Serializes RelationshipTarget as serde externally-tagged: {"Index":n} or {"Key":"s"}.</summary>
public sealed class RelationshipTargetConverter : JsonConverter<RelationshipTarget>
{
    public override RelationshipTarget Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        var root = doc.RootElement;
        if (root.TryGetProperty("Index", out var idx))
            return RelationshipTarget.FromIndex(idx.GetUInt32());
        if (root.TryGetProperty("Key", out var key))
            return RelationshipTarget.FromKey(key.GetString()!);
        throw new JsonException("Invalid RelationshipTarget");
    }

    public override void Write(Utf8JsonWriter writer, RelationshipTarget value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        if (value.IsIndex)
            writer.WriteNumber("Index", value.Index!.Value);
        else
            writer.WriteString("Key", value.Key!);
        writer.WriteEndObject();
    }
}

public sealed class Relationship
{
    public uint Source { get; set; }
    public RelationshipTarget Target { get; set; } = RelationshipTarget.FromIndex(0);
    public RelationshipKind Kind { get; set; }
}

/// <summary>A single element in the internal flat document.</summary>
public sealed class InternalElement
{
    public InternalElementId Id { get; set; }
    public ElementKind Kind { get; set; }
    public string Text { get; set; } = "";
    public ushort Depth { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public uint? Page { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public BoundingBox? Bbox { get; set; }

    public ContentLayer Layer { get; set; } = ContentLayer.Body;

    public List<TextAnnotation> Annotations { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public Dictionary<string, string>? Attributes { get; set; }

    /// <summary>
    /// Attribute key holding a list item's literal source marker text (e.g. <c>"B."</c>,
    /// <c>"(a)"</c>, <c>"iv."</c>).
    /// </summary>
    /// <remarks>
    /// Deliberately a generic attribute rather than a field on <c>ElementKind.ListItem</c>, which
    /// is a value type matched by value throughout the renderers and the structure-tree
    /// derivation. It is also intentionally left out of the public-attribute filter, so it
    /// reaches the public <c>DocumentStructure</c> tree via a node's attributes with no change
    /// needed in the derivation.
    /// </remarks>
    public const string ListItemSourceLabelAttribute = "list_marker";

    /// <summary>
    /// The literal source list-marker text, if one was captured.
    /// </summary>
    /// <remarks>
    /// <c>null</c> for every non-PDF extractor and for PDF list items whose marker text was not
    /// confidently recovered — renderers fall back to the synthesized sequence position there,
    /// exactly as they did before this attribute existed.
    /// </remarks>
    public string? ListItemSourceLabel =>
        Attributes is not null && Attributes.TryGetValue(ListItemSourceLabelAttribute, out var label)
            ? label
            : null;

    /// <summary>
    /// Attach this element's literal source marker text. An empty label is a caller bug (a
    /// marker-strip that removed nothing), not a real source marker, so it is ignored rather than
    /// manufacturing a spurious attribute.
    /// </summary>
    public void SetListItemSourceLabel(string label)
    {
        if (label.Length == 0) return;
        (Attributes ??= new Dictionary<string, string>(StringComparer.Ordinal))
            [ListItemSourceLabelAttribute] = label;
    }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Anchor { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public object? OcrGeometry { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public object? OcrConfidence { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public object? OcrRotation { get; set; }

    /// <summary>Create a simple text element with minimal fields (index 0 for ID).</summary>
    public static InternalElement TextElement(ElementKind kind, string text, ushort depth)
    {
        return new InternalElement
        {
            Id = InternalElementId.Generate(kind.Discriminant(), text, null, 0),
            Kind = kind,
            Text = text,
            Depth = depth,
        };
    }
}

/// <summary>The internal flat document representation produced by every extractor.</summary>
public sealed class InternalDocument
{
    public List<InternalElement> Elements { get; set; } = new();
    public List<Relationship> Relationships { get; set; } = new();
    public string SourceFormat { get; set; } = "";
    public Metadata Metadata { get; set; } = new();
    public List<ExtractedImage> Images { get; set; } = new();
    public List<Table> Tables { get; set; } = new();
    public List<ExtractedUri> Uris { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<ArchiveEntry>? Children { get; set; }

    /// <summary>See <see cref="ExtractedDocument.Attachments"/>.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<ExtractedAttachment>? Attachments { get; set; }

    public string MimeType { get; set; } = "application/octet-stream";

    public List<ProcessingWarning> ProcessingWarnings { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<PdfAnnotation>? Annotations { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<PageContent>? PrebuiltPages { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? PreRenderedContent { get; set; }

    [JsonIgnore] public List<object>? Revisions { get; set; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] public List<PdfFormField>? FormFields { get; set; }
    [JsonIgnore] public List<object> Formulas { get; set; } = new();

    private const int MaxUris = 100_000;

    public InternalDocument() { }

    public InternalDocument(string sourceFormat) => SourceFormat = sourceFormat;

    public uint PushElement(InternalElement element)
    {
        uint idx = (uint)Elements.Count;
        Elements.Add(element);
        return idx;
    }

    public void PushRelationship(Relationship r) => Relationships.Add(r);

    public uint PushTable(Table table)
    {
        uint idx = (uint)Tables.Count;
        Tables.Add(table);
        return idx;
    }

    public uint PushImage(ExtractedImage image)
    {
        uint idx = (uint)Images.Count;
        Images.Add(image);
        return idx;
    }

    public void PushUri(ExtractedUri uri)
    {
        if (Uris.Count < MaxUris)
            Uris.Add(uri);
    }
}
