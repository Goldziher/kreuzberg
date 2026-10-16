// Photoshop PSD/PSB extractor. No Rust source: this format is an X-Ray addition beyond upstream
// xberg. Upstream's MIME sniffing (the `infer` crate) does recognise the `8BPS` signature as
// `image/vnd.adobe.photoshop`, and `mime_guess` maps `.psd` to it, but no upstream extractor claims
// that type, so xberg answers a PSD with an unsupported-format error.
//
// The file is read by the X-Ray.Psd package (namespace `XRay.Psd`, ported from the Patchy image
// editor), whose core is pure managed with no dependencies. Only its structural API is used —
// the layer tree, type-layer text, image resources and embedded smart objects. Pixel data stays
// compressed: nothing here renders, composites or decodes a channel, which is what keeps
// extracting a multi-gigabyte PSB cheap.

using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.Exif;
using XRay.Content.Types;
using XRay.Psd;
using XRay.Psd.Layers;
using PsdText = XRay.Psd.Text;

namespace XRay.Content.Extractors;

/// <summary>
/// Photoshop document extractor (<c>.psd</c>, <c>.psb</c>): the layer tree as headings, each type
/// layer's text as paragraphs beneath its layer, and document facts as metadata.
/// </summary>
/// <remarks>
/// <para>
/// The document is laid out the way the Layers panel shows it, top to bottom. The file name (or
/// the XMP/IPTC title, when the document carries one) is the <see cref="ElementKind.Title"/>; each
/// group and layer below it is a heading one level deeper than its parent, so a top-level layer is
/// an <c>h2</c>. Markdown has six heading levels and a layer tree has no limit, so anything nested
/// deeper than an <c>h6</c> stays at <c>h6</c> rather than turning into a different element: the
/// heading-driven JSON tree keeps every layer a section, at the cost of deep siblings and children
/// sharing a level. Without a title (bytes with no file name and no metadata title), top-level
/// layers start at <c>h1</c>.
/// </para>
/// <para>
/// A layer heading is written for every layer, pixel and adjustment layers included, so the
/// structure is visible even where there is no text. A type layer's text follows its heading,
/// one paragraph per Photoshop paragraph. Where the document-level <c>Txt2</c> text engine holds a
/// different version of a layer's text, that version is the one written, because it is the one
/// Photoshop displays; a <c>Txt2</c> text object no layer claims is written directly under the
/// document title. An embedded PSD/PSB smart object's own layer tree is nested under the smart
/// object layer's heading, to <see cref="MaxEmbeddedDepth"/> levels.
/// </para>
/// <para>
/// Hidden layers are included: extraction is about what the file contains, not what a render of
/// it shows. Channel names, path names, slices, layer comps, fonts and linked file names are
/// document facts rather than body text and go into <see cref="Metadata.Additional"/>; XMP and
/// IPTC fields map onto <see cref="Metadata"/>'s own title, subject, authors and keywords, and a
/// slice's URL becomes an <see cref="ExtractedUri"/>.
/// </para>
/// </remarks>
public sealed class PsdExtractor : IExtractor
{
    /// <summary>The MIME type <c>infer</c> and <c>mime_guess</c> assign to Photoshop documents.</summary>
    public const string MimeType = "image/vnd.adobe.photoshop";

    /// <summary>How deep embedded smart objects are followed, matching X-Ray.Psd's own default.</summary>
    private const int MaxEmbeddedDepth = 4;

    private const byte MaxHeadingLevel = 6;

    /// <summary>Photoshop's EXIF resource (EXIF data 1): a bare TIFF-structured block.</summary>
    private const ushort ExifResourceId = 1058;

    public IEnumerable<string> SupportedMimeTypes => new[]
    {
        MimeType,
        "image/x-photoshop",
        "application/x-photoshop",
        "application/photoshop",
        "application/psd",
        "image/psd",
    };

    public int Priority => 50;

    /// <summary>Only the structure is needed; colour management only matters to pixel decoding.</summary>
    private static readonly PsdLoadOptions LoadOptions = new() { ColorManagement = false };

    /// <summary>
    /// What <see cref="PsdDocument.ExtractText"/> is asked for: only the <c>Txt2</c> text objects
    /// (which need its owner-matching logic) and the XMP/IPTC fields. Layer names and type-layer
    /// text come from walking the tree directly, which keeps their nesting.
    /// </summary>
    private static readonly PsdText.TextExtractionOptions TextOptions = new()
    {
        IncludeLayerNames = false,
        IncludeTextLayers = true,
        IncludeTextEngineObjects = true,
        IncludeChannelNames = false,
        IncludePathNames = false,
        IncludeSlices = false,
        IncludeMetadata = true,
        IncludeHiddenLayers = true,
        MaxEmbeddedDepth = 0,
    };

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        PsdDocument document;
        Cancel.ThrowIfRequested();
        try
        {
            document = PsdDocument.Load(content.ToArray(), LoadOptions);
        }
        catch (PsdFormatException e)
        {
            throw new InvalidDataException($"psd parse failed: {e.Message}", e);
        }

        Cancel.ThrowIfRequested();
        var builder = new InternalDocumentBuilder("psd");
        var facts = new DocumentFacts();
        var text = TextOf(document);
        Cancel.ThrowIfRequested();
        var metadata = BuildMetadata(document, text, config, facts, builder);

        byte baseLevel = 1;
        if (metadata.Title is { Length: > 0 } title)
        {
            builder.PushTitle(title, null, null);
            baseLevel = 2;
        }

        WriteDocument(builder, document, text, baseLevel, 0, facts);
        FinishMetadata(metadata, facts);

        var doc = builder.Build();
        doc.MimeType = mimeType;
        doc.Metadata = metadata;
        return doc;
    }

    // --- body ---

    /// <summary>Text the tree walk cannot see by itself, from one document's <c>Txt2</c> block.</summary>
    private sealed record DocumentText(
        Dictionary<PsdLayer, string> DisplayedText,
        List<string> UnownedText,
        List<PsdText.PsdTextItem> Metadata);

    private static DocumentText TextOf(PsdDocument document)
    {
        var displayed = new Dictionary<PsdLayer, string>(ReferenceEqualityComparer.Instance);
        var unowned = new List<string>();
        var metadata = new List<PsdText.PsdTextItem>();
        foreach (var item in document.ExtractText(TextOptions).Items)
        {
            switch (item.Kind)
            {
                // Reported only where Txt2 disagrees with the layer's own record, or no layer
                // owns the object at all.
                case PsdText.PsdTextKind.TextEngineObject when item.Layer is { } owner:
                    displayed.TryAdd(owner, item.Text);
                    break;
                case PsdText.PsdTextKind.TextEngineObject:
                    unowned.Add(item.Text);
                    break;
                case PsdText.PsdTextKind.Metadata:
                    metadata.Add(item);
                    break;
            }
        }
        return new DocumentText(displayed, unowned, metadata);
    }

    private static void WriteDocument(
        InternalDocumentBuilder builder, PsdDocument document, DocumentText text, byte level, int embeddedDepth,
        DocumentFacts facts)
    {
        // Text no layer owns belongs to the document itself, so it sits under the heading this
        // document's layers hang from: the title, or the smart object layer that embeds it.
        foreach (var orphan in text.UnownedText) PushParagraphs(builder, orphan, (ushort)(level - 1));

        // RootLayers is bottom-to-top, as stored; the panel (and reading order) is top-down.
        var roots = document.RootLayers;
        for (int i = roots.Count - 1; i >= 0; i--)
            WriteLayer(builder, document, roots[i], text, level, embeddedDepth, facts);
    }

    private static void WriteLayer(
        InternalDocumentBuilder builder, PsdDocument document, PsdLayer layer, DocumentText text, byte level,
        int embeddedDepth, DocumentFacts facts)
    {
        // Between layers: X-Ray.Psd's own loops are not interruptible until it ships overloads
        // taking a token, so a layer tree is the finest grain available here.
        Cancel.ThrowIfRequested();
        bool top = embeddedDepth == 0;
        if (top) facts.LayerCount++;
        bool named = layer.Name.Length > 0;
        if (named) builder.PushHeading(level, layer.Name, null, null);

        string? body = text.DisplayedText.TryGetValue(layer, out var displayed) ? displayed : layer.Text?.Text;
        if (body is { Length: > 0 })
        {
            if (top) facts.TextLayerCount++;
            PushParagraphs(builder, body, named ? level : (ushort)(level - 1));
        }
        if (layer.Text is { } info)
            foreach (var font in info.Fonts) facts.Fonts.Add(font);

        byte childLevel = (byte)Math.Min(level + 1, MaxHeadingLevel);
        if (layer.SmartObjectFileId is { } fileId)
            WriteSmartObject(builder, document, fileId, childLevel, embeddedDepth, facts);

        var children = layer.Children;
        for (int i = children.Count - 1; i >= 0; i--)
            WriteLayer(builder, document, children[i], text, childLevel, embeddedDepth, facts);
    }

    /// <summary>An embedded PSD/PSB smart object contributes its own layer tree, one level down.</summary>
    private static void WriteSmartObject(
        InternalDocumentBuilder builder, PsdDocument document, string fileId, byte level, int embeddedDepth,
        DocumentFacts facts)
    {
        var file = document.LinkedFiles.FirstOrDefault(f => f.Id == fileId);
        if (file is null) return;
        if (file.FileName.Length > 0) facts.LinkedFiles.Add(file.FileName);
        if (!file.IsPhotoshopDocument || embeddedDepth >= MaxEmbeddedDepth) return;

        PsdDocument embedded;
        try
        {
            embedded = PsdDocument.Load(file.Data, LoadOptions);
        }
        catch (PsdFormatException e)
        {
            // A damaged embedded document costs its own text, never the document around it.
            builder.AddWarning(new ProcessingWarning
            {
                Source = "psd",
                Message = $"embedded smart object '{file.FileName}' could not be read: {e.Message}",
            });
            return;
        }

        WriteDocument(builder, embedded, TextOf(embedded), level, embeddedDepth + 1, facts);
    }

    /// <summary>One paragraph per Photoshop paragraph; the library has normalized breaks to <c>\n</c>.</summary>
    /// <remarks>
    /// A heading of level <c>n</c> sits at depth <c>n - 1</c>, so a paragraph at depth <c>n</c>
    /// belongs to it: the document structure nests it in that heading's section instead of
    /// popping back to the root, and the plain renderer indents it one step past the heading.
    /// The builder places every paragraph at its own container depth, which is 0 here, so the
    /// depth is set on the element after it is pushed; the element ID does not depend on it.
    /// </remarks>
    private static void PushParagraphs(InternalDocumentBuilder builder, string text, ushort depth)
    {
        foreach (var paragraph in text.Split('\n'))
        {
            string trimmed = paragraph.Trim();
            if (trimmed.Length == 0) continue;
            uint index = builder.PushParagraph(trimmed, new(), null, null);
            builder.Build().Elements[(int)index].Depth = depth;
        }
    }

    // --- metadata ---

    /// <summary>
    /// What the tree walk learns on its way through, for the metadata written after it. The counts
    /// are the outer document's own; fonts and linked files include those of embedded documents.
    /// </summary>
    private sealed class DocumentFacts
    {
        public int LayerCount;
        public int TextLayerCount;
        public readonly SortedSet<string> Fonts = new(StringComparer.Ordinal);
        public readonly List<string> LinkedFiles = new();
    }

    private static Metadata BuildMetadata(
        PsdDocument document, DocumentText text, ExtractionConfig config, DocumentFacts facts,
        InternalDocumentBuilder builder)
    {
        var metadata = new Metadata
        {
            Format = FormatMetadata.Image(new ImageMetadata
            {
                Width = (uint)Math.Max(0, document.Width),
                Height = (uint)Math.Max(0, document.Height),
                Format = document.IsLargeDocument ? "PSB" : "PSD",
                Exif = ExifOf(document),
            }),
        };

        // X-Ray.Psd reports each field under its source (`xmp/title`, `iptc/byline`, ...); XMP
        // comes first, so it wins where both carry a field.
        string? Field(params string[] sources) =>
            text.Metadata.FirstOrDefault(i => sources.Contains(i.Source))?.Text;
        List<string>? Fields(params string[] sources)
        {
            var values = text.Metadata.Where(i => sources.Contains(i.Source)).Select(i => i.Text)
                .Distinct(StringComparer.Ordinal).ToList();
            return values.Count > 0 ? values : null;
        }

        metadata.Title = Field("xmp/title", "iptc/title")
            ?? (config.SourceName is { Length: > 0 } name ? Path.GetFileNameWithoutExtension(name) : null);
        metadata.Subject = Field("xmp/description", "iptc/caption");
        metadata.Authors = Fields("xmp/creator", "iptc/byline");
        metadata.Keywords = Fields("xmp/keywords", "iptc/keywords");
        if (Field("xmp/headline", "iptc/headline") is { } headline) Add(metadata, "headline", headline);
        if (Field("xmp/rights", "iptc/copyright") is { } rights) Add(metadata, "copyright", rights);

        Add(metadata, "color_mode", ColorModeName(document.ColorMode));
        Add(metadata, "bit_depth", document.Depth);
        Add(metadata, "channel_count", document.ChannelCount);

        var resources = document.Resources;
        if (resources.Resolution is { } resolution)
        {
            Add(metadata, "dpi_x", resolution.HorizontalPpi);
            Add(metadata, "dpi_y", resolution.VerticalPpi);
        }
        if (resources.IccProfile is { Description.Length: > 0 } icc) Add(metadata, "icc_profile", icc.Description);
        if (resources.VersionInfo is { WriterName.Length: > 0 } version) Add(metadata, "application", version.WriterName);

        if (resources.ChannelNames is { Count: > 0 } channels) Add(metadata, "channel_names", channels);

        var paths = document.ImageResources
            .Where(r => r.Id is >= ImageResourceIds.PathFirst and <= ImageResourceIds.PathLast && r.Name.Length > 0)
            .Select(r => r.Name)
            .ToList();
        if (paths.Count > 0) Add(metadata, "path_names", paths);

        if (resources.LayerComps is { Comps.Count: > 0 } comps)
            Add(metadata, "layer_comps", comps.Comps.Select(c => c.Name).Where(n => n.Length > 0).ToList());

        if (resources.Slices is { Slices.Count: > 0 } slices)
        {
            var sliceEntries = new List<Dictionary<string, string>>();
            foreach (var slice in slices.Slices)
            {
                var entry = new Dictionary<string, string>(StringComparer.Ordinal);
                void Put(string key, string value) { if (value.Length > 0) entry[key] = value; }
                Put("name", slice.Name);
                Put("url", slice.Url);
                Put("message", slice.Message);
                Put("alt", slice.AltTag);
                Put("cell_text", slice.CellText);
                if (entry.Count > 0) sliceEntries.Add(entry);

                if (slice.Url.Length > 0)
                {
                    builder.PushUri(new ExtractedUri
                    {
                        Url = slice.Url,
                        Label = slice.AltTag.Length > 0 ? slice.AltTag : slice.Name.Length > 0 ? slice.Name : null,
                        Kind = UriKind.Hyperlink,
                    });
                }
            }
            if (sliceEntries.Count > 0) Add(metadata, "slices", sliceEntries);
        }

        return metadata;
    }

    private static void FinishMetadata(Metadata metadata, DocumentFacts facts)
    {
        Add(metadata, "layer_count", facts.LayerCount);
        Add(metadata, "text_layer_count", facts.TextLayerCount);
        if (facts.Fonts.Count > 0) Add(metadata, "fonts", facts.Fonts.ToList());
        if (facts.LinkedFiles.Count > 0)
            Add(metadata, "linked_files", facts.LinkedFiles.Distinct(StringComparer.Ordinal).ToList());
    }

    private static void Add<T>(Metadata metadata, string key, T value) =>
        metadata.Additional[key] = JsonSerializer.SerializeToElement(value, Json.Options);

    /// <summary>The EXIF block Photoshop stores as resource 1058, read the way an image file's is.</summary>
    private static Dictionary<string, string> ExifOf(PsdDocument document)
    {
        if (document.GetImageResource(ExifResourceId) is not { Data.Length: > 0 } resource) return new();
        try
        {
            return ExifReader.Extract(new SixLabors.ImageSharp.Metadata.Profiles.Exif.ExifProfile(resource.Data.ToArray()));
        }
        catch (Exception e) when (e is ArgumentException or IndexOutOfRangeException or InvalidOperationException)
        {
            return new();
        }
    }

    private static string ColorModeName(PsdColorMode mode) => mode switch
    {
        PsdColorMode.Bitmap => "bitmap",
        PsdColorMode.Grayscale => "grayscale",
        PsdColorMode.Indexed => "indexed",
        PsdColorMode.Rgb => "rgb",
        PsdColorMode.Cmyk => "cmyk",
        PsdColorMode.Multichannel => "multichannel",
        PsdColorMode.Duotone => "duotone",
        PsdColorMode.Lab => "lab",
        _ => ((int)mode).ToString(System.Globalization.CultureInfo.InvariantCulture),
    };
}
