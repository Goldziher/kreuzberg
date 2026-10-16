// Ported from crates/xberg/src/types/annotations.rs, plus the fields a consumer of this port
// asked for beyond upstream's model (flags, creation date, link destination, page size).
using System.Text.Json.Serialization;

namespace XRay.Content.Types;

/// <summary>Type of PDF annotation. Serialized as a bare snake_case string.</summary>
public enum PdfAnnotationType
{
    /// <summary>Sticky note or free-text annotation.</summary>
    Text,
    Highlight,
    Link,
    Stamp,
    Underline,
    StrikeOut,
    Squiggly,
    Ink,
    Square,
    Circle,
    Polygon,
    PolyLine,
    Line,
    Caret,
    FileAttachment,
    Sound,
    Movie,
    Other,
}

/// <summary>A PDF annotation extracted from a document page.</summary>
public sealed class PdfAnnotation
{
    public PdfAnnotationType AnnotationType { get; set; }

    /// <summary>
    /// The PDF <c>/Subtype</c> exactly as written (<c>FreeText</c>, <c>Caret</c>, <c>Redact</c>…),
    /// which <see cref="AnnotationType"/> collapses (a free-text box is <c>Text</c>).
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Subtype { get; set; }

    /// <summary>Text content of the annotation: the comment, or the URI of a link.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Content { get; set; }

    /// <summary>Page the annotation is on, 1-based.</summary>
    public uint PageNumber { get; set; }

    /// <summary>The annotation's <c>/Rect</c> in default user space (origin bottom-left).</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public BoundingBox? BoundingBox { get; set; }

    /// <summary>Author (<c>/T</c>).</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Author { get; set; }

    /// <summary>Last modification date (<c>/M</c>) as the raw PDF date string.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Modified { get; set; }

    /// <summary>Creation date (<c>/CreationDate</c>) as the raw PDF date string.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Created { get; set; }

    /// <summary>Colour (<c>/C</c>) as <c>#rrggbb</c>; gray and CMYK are converted.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Color { get; set; }

    /// <summary>Subject (<c>/Subj</c>).</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Subject { get; set; }

    /// <summary>Per-line boxes from <c>/QuadPoints</c>, for the four text-markup types.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public List<BoundingBox>? QuadPoints { get; set; }

    /// <summary>The page text under <see cref="QuadPoints"/>, when it could be recovered.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? MarkedText { get; set; }

    /// <summary>The <c>/F</c> flag bits (1 invisible, 2 hidden, 4 print, 32 no-view…).</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public uint? Flags { get; set; }

    /// <summary>
    /// For a link that points inside the document, the destination as the anchor URI
    /// (<c>#name</c> or <c>#page=N</c>) that also lands in <see cref="ExtractedDocument.Uris"/>.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Destination { get; set; }

    /// <summary>Width and height of the page's visible box, to normalise <see cref="BoundingBox"/> against.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public double? PageWidth { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public double? PageHeight { get; set; }

    /// <summary>
    /// The page's visible box (crop box, else media box) in default user space: the origin to subtract
    /// from <see cref="BoundingBox"/> to place it on the page.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public BoundingBox? PageBox { get; set; }

    /// <summary>The page's <c>/Rotate</c> (0, 90, 180 or 270). <see cref="BoundingBox"/> is in the unrotated space.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public int? PageRotation { get; set; }

    /// <summary>
    /// Whether a viewer would draw it: not invisible, hidden or no-view, and not zero opacity.
    /// Most consumers want only these.
    /// </summary>
    [JsonIgnore]
    public bool IsVisible => Flags is not { } f || (f & (1u | 2u | 32u)) == 0;
}

/// <summary>
/// A file attached to a PDF: an entry of the <c>/EmbeddedFiles</c> name tree, a file-attachment
/// annotation or a member of a PDF portfolio. The same file is also extracted into
/// <see cref="ExtractedDocument.Children"/> when its type is one this package reads.
/// </summary>
public sealed class ExtractedAttachment
{
    /// <summary>File name as the filespec gives it (<c>/UF</c>, else <c>/F</c>, else the tree key).</summary>
    public string Name { get; set; } = "";

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? MimeType { get; set; }

    /// <summary>Decompressed size in bytes.</summary>
    public long Size { get; set; }

    /// <summary>Size of the stream as stored, before its filters.</summary>
    public long CompressedSize { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Description { get; set; }

    /// <summary><c>/AFRelationship</c> (Source, Data, Alternative…), when the filespec has one.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Relationship { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Created { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Modified { get; set; }

    /// <summary><c>name_tree</c>, <c>file_attachment_annotation</c> or <c>portfolio</c>.</summary>
    public string Source { get; set; } = "name_tree";

    /// <summary>Page of the file-attachment annotation, 1-based.</summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public uint? PageNumber { get; set; }

    /// <summary>
    /// Why the file was not extracted into <c>Children</c> (a size or ratio limit, an unknown
    /// type, a recursion limit); null when it was, or when it was skipped for no reason worth
    /// reporting.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? SkippedReason { get; set; }

    /// <summary>
    /// The file's bytes, only when <see cref="Core.PdfOptions.AttachEmbeddedFileBytes"/> asked for
    /// them. Never serialized.
    /// </summary>
    [JsonIgnore]
    public byte[]? Data { get; set; }
}
