// Ported from crates/xberg/src/extractors/epub/metadata.rs
// OPF (Open Packaging Format) parsing: Dublin Core metadata, manifest, spine order.

using System.Text;
using System.Text.Json;
using System.Xml.Linq;
using XRay.Content.Types;

namespace XRay.Content.Internal.Epub;

/// <summary>Metadata extracted from the OPF file. Mirrors Rust `OepbMetadata`.</summary>
internal sealed class OpfMetadata
{
    public string? Title { get; set; }

    /// <summary>Every <c>dc:creator</c>, in document order. They become the authors.</summary>
    public List<string> Creators { get; } = new();
    public string? Date { get; set; }
    public string? Language { get; set; }
    public string? Identifier { get; set; }
    public string? Publisher { get; set; }
    /// <summary>Every <c>dc:subject</c>, in document order. They become the keywords.</summary>
    public List<string> Subjects { get; } = new();
    public string? Description { get; set; }
    public string? Rights { get; set; }
    public string? Coverage { get; set; }
    public string? Format { get; set; }
    public string? Relation { get; set; }
    public string? Source { get; set; }
    public string? DcType { get; set; }
    public string? CoverImageHref { get; set; }
}

/// <summary>A spine entry (`&lt;itemref idref="..."/&gt;`). Mirrors Rust `EpubSpineItem`.</summary>
internal sealed class EpubSpineItem
{
    public string Idref { get; set; } = "";
}

/// <summary>A manifest entry. Mirrors Rust `ManifestItem`.</summary>
internal sealed class ManifestItem
{
    public string RawHref { get; set; } = "";
    public string? Path { get; set; }
    public string? PathResolutionError { get; set; }
    public string? MediaType { get; set; }
    public string? Fallback { get; set; }
    public string? Properties { get; set; }

    /// <summary>Mirrors `is_renderable_body_document`.</summary>
    /// <remarks>
    /// <c>text/html</c> is not an EPUB core media type, but real-world EPUB 3 files (Internet
    /// Archive builds, for one) declare every page with it while the payload is XHTML. The spine
    /// loop parses the payload as XML either way, so accepting the label costs nothing and rescues
    /// whole books — an exact-string match against <c>application/xhtml+xml</c> alone extracted
    /// them with zero content. The same match also rejected <c>text/xml</c>, a type carrying
    /// parameters, and any uppercase letter (upstream #1486).
    /// </remarks>
    public bool IsRenderableBodyDocument()
    {
        string? mediaType = NormalizedMediaType();
        return mediaType is null
            ? HasRenderableExtension(RawHref)
            : mediaType is "application/xhtml+xml" or "application/x-dtbook+xml"
                          or "text/html" or "text/xml" or "application/xml";
    }

    /// <summary>
    /// The media type without parameters, whitespace or case. <c>null</c> when the attribute is
    /// missing or empty, so the file extension decides.
    /// </summary>
    private string? NormalizedMediaType()
    {
        if (MediaType is null) return null;
        int semicolon = MediaType.IndexOf(';');
        string normalized = (semicolon >= 0 ? MediaType[..semicolon] : MediaType).Trim().ToLowerInvariant();
        return normalized.Length == 0 ? null : normalized;
    }

    /// <summary>Whether the item carries the EPUB 3 <c>nav</c> property.</summary>
    public bool IsNav() => HasProperty("nav");

    /// <summary>
    /// True when the item is an image by media type, or by extension when the media type is
    /// missing. The cover has to satisfy this: some producers point <c>&lt;meta name="cover"&gt;</c>
    /// at the cover XHTML page instead of the image.
    /// </summary>
    public bool IsImage()
    {
        if (MediaType is not null)
            return MediaType.Trim().StartsWith("image/", StringComparison.OrdinalIgnoreCase);
        int dot = RawHref.LastIndexOf('.');
        if (dot < 0) return false;
        return RawHref[(dot + 1)..].ToLowerInvariant()
            is "jpg" or "jpeg" or "png" or "gif" or "webp" or "svg" or "bmp";
    }

    /// <summary>Whether <paramref name="property"/> appears in the item's space-separated
    /// <c>properties</c> attribute.</summary>
    public bool HasProperty(string property) =>
        Properties is not null
        && Properties.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries)
                     .Any(value => string.Equals(value, property, StringComparison.OrdinalIgnoreCase));

    /// <summary>
    /// True when the item is an SVG content document. EPUB 3 allows one in the spine; it is
    /// rendered through the SVG text walk when no XHTML fallback exists.
    /// </summary>
    public bool IsSvg() =>
        MediaType is not null
        && MediaType.Trim().Equals("image/svg+xml", StringComparison.OrdinalIgnoreCase);

    /// <summary>Mirrors `resolved_path`: the resolved path, or null with an error message.</summary>
    public (string? Path, string Error) ResolvedPath()
    {
        if (Path is not null) return (Path, "");
        return (null, PathResolutionError ?? $"unable to resolve manifest href '{RawHref}'");
    }

    private static bool HasRenderableExtension(string href)
    {
        int hash = href.IndexOf('#');
        string path = hash >= 0 ? href.Substring(0, hash) : href;
        int slash = path.LastIndexOf('/');
        if (slash >= 0) path = path.Substring(slash + 1);
        int dot = path.LastIndexOf('.');
        if (dot < 0) return false;
        string ext = path.Substring(dot + 1).ToLowerInvariant();
        return ext is "xhtml" or "html" or "htm" or "xml" or "dtbook";
    }
}

/// <summary>Parsed OPF package document. Mirrors Rust `EpubPackageDocument`.</summary>
internal sealed class EpubPackageDocument
{
    public OpfMetadata Metadata { get; set; } = new();
    public Dictionary<string, ManifestItem> Manifest { get; set; } = new(StringComparer.Ordinal);
    public List<EpubSpineItem> SpineItems { get; set; } = new();
    public HashSet<string> GuideTocPaths { get; set; } = new(StringComparer.Ordinal);

    public bool IsGuideTocCandidatePath(string path) => GuideTocPaths.Contains(path);
}

internal static class EpubOpf
{
    /// <summary>Parse the OPF file, extracting metadata, manifest and spine order. Mirrors `parse_opf`.</summary>
    public static (EpubPackageDocument Package, List<ProcessingWarning> Warnings) ParseOpf(string xml, string opfDir)
    {
        XDocument doc;
        try
        {
            doc = XDocument.Parse(xml, LoadOptions.PreserveWhitespace);
        }
        catch (Exception e) when (Cancel.NotCancelling)
        {
            throw new EpubParseException($"Failed to parse OPF file: {e.Message}");
        }

        var warnings = new List<ProcessingWarning>();
        var package = new EpubPackageDocument();
        var manifest = new Dictionary<string, ManifestItem>(StringComparer.Ordinal);
        var meta = package.Metadata;

        // <dc:date> is qualified by opf:event in EPUB 2; a modification date is used only when
        // no other date exists. <dc:title> can appear more than once (subtitles, series names),
        // and EPUB 3 marks the main one with a refining <meta property="title-type">main</meta>.
        var dates = new List<(string? Event, string Value)>();
        var titles = new List<(string? Id, string Value)>();
        string? mainTitleId = null;
        string? uniqueIdentifierId = doc.Descendants()
            .FirstOrDefault(n => n.Name.LocalName == "package")?.Attribute("unique-identifier")?.Value;

        foreach (var node in doc.Descendants())
        {
            // Dublin Core elements inside an EPUB 3 <collection> describe the collection, not the
            // book, so they are not package metadata. Matching on the namespace rather than the
            // local name alone is what keeps a bare <title> from overwriting the book's.
            if (IsDublinCore(node))
            {
                string? text = DirectText(node);
                if (text is null || text.Length == 0) continue;

                // Every arm used to overwrite the previous value, so a subtitle replaced the
                // title, an illustrator replaced the author, and a modification date replaced the
                // publication date. Repeatable fields accumulate; single-valued ones keep the
                // first they see.
                switch (node.Name.LocalName.ToLowerInvariant())
                {
                    case "title": titles.Add((node.Attribute("id")?.Value, text)); break;
                    case "creator": meta.Creators.Add(text); break;
                    case "date":
                        dates.Add((node.Attributes().FirstOrDefault(a => a.Name.LocalName == "event")
                                       ?.Value.ToLowerInvariant(),
                                   text));
                        break;
                    case "language": meta.Language ??= text; break;
                    case "identifier":
                        // The package's own unique-identifier wins outright; otherwise first wins.
                        if (node.Attribute("id")?.Value is { } id && uniqueIdentifierId is not null
                            && string.Equals(id, uniqueIdentifierId, StringComparison.Ordinal))
                            meta.Identifier = text;
                        else
                            meta.Identifier ??= text;
                        break;
                    case "publisher": meta.Publisher ??= text; break;
                    case "subject": meta.Subjects.Add(text); break;
                    case "description": meta.Description ??= text; break;
                    case "rights": meta.Rights ??= text; break;
                    case "coverage": meta.Coverage ??= text; break;
                    case "format": meta.Format ??= text; break;
                    case "relation": meta.Relation ??= text; break;
                    case "source": meta.Source ??= text; break;
                    case "type": meta.DcType ??= text; break;
                }
                continue;
            }

            switch (node.Name.LocalName)
            {
                case "meta":
                    // EPUB 3 marks the main title with a refining meta element.
                    if (node.Attribute("property")?.Value == "title-type"
                        && DirectText(node) == "main"
                        && node.Attribute("refines")?.Value is { } refines
                        && refines.StartsWith('#'))
                        mainTitleId ??= refines[1..];
                    break;
                case "item":
                {
                    string? id = node.Attribute("id")?.Value;
                    string? href = node.Attribute("href")?.Value;
                    if (id is not null && href is not null)
                    {
                        string? path;
                        string? resolutionError;
                        if (EpubContainer.TryResolvePath(opfDir, href, out var resolved, out var err))
                        {
                            path = resolved.Path;
                            resolutionError = null;
                        }
                        else
                        {
                            path = null;
                            resolutionError = err;
                        }
                        manifest[id] = new ManifestItem
                        {
                            RawHref = href,
                            Path = path,
                            PathResolutionError = resolutionError,
                            MediaType = node.Attribute("media-type")?.Value,
                            Fallback = node.Attribute("fallback")?.Value,
                            Properties = node.Attribute("properties")?.Value,
                        };
                    }
                    break;
                }
                case "reference":
                {
                    var kind = node.Attribute("type")?.Value;
                    string? href = node.Attribute("href")?.Value;
                    if (kind is not null && kind.Equals("toc", StringComparison.OrdinalIgnoreCase) && href is not null)
                    {
                        if (EpubContainer.TryResolvePath(opfDir, href, out var resolved, out var err))
                            package.GuideTocPaths.Add(resolved.Path);
                        else
                            warnings.Add(new ProcessingWarning
                            {
                                Source = "epub",
                                Message = $"Skipping malformed guide reference '{href}': {err}",
                            });
                    }
                    break;
                }
            }
        }

        meta.Date = SelectPublicationDate(dates);
        // The main title wins if a refining meta named one; otherwise the first title does.
        meta.Title = titles.FirstOrDefault(t => t.Id is not null && t.Id == mainTitleId).Value
                     ?? (titles.Count > 0 ? titles[0].Value : null);

        // EPUB 3 marks the cover with a manifest property; EPUB 2 points at it from a meta
        // element. Either way the item has to be an image: some producers point the meta at the
        // cover XHTML page instead.
        ManifestItem? cover = manifest.Values.FirstOrDefault(i => i.HasProperty("cover-image") && i.IsImage());
        if (cover is null)
        {
            string? coverItemId = doc.Descendants()
                .FirstOrDefault(n => n.Name.LocalName == "meta" && n.Attribute("name")?.Value == "cover")
                ?.Attribute("content")?.Value;
            if (coverItemId is not null && manifest.TryGetValue(coverItemId, out var fromMeta) && fromMeta.IsImage())
                cover = fromMeta;
        }
        if (cover is not null && cover.ResolvedPath().Path is { } coverPath)
            meta.CoverImageHref = coverPath;

        // Spine order (document order of <itemref idref="..."/>).
        foreach (var node in doc.Descendants())
        {
            if (node.Name.LocalName == "itemref")
            {
                var idref = node.Attribute("idref")?.Value;
                if (idref is not null)
                    package.SpineItems.Add(new EpubSpineItem { Idref = idref });
            }
        }

        package.Manifest = manifest;
        return (package, warnings);
    }

    /// <summary>
    /// Standard Dublin Core fields surfaced into the generic metadata map. Mirrors
    /// `build_additional_metadata`. Values are JSON strings.
    /// </summary>
    public static Dictionary<string, JsonElement> BuildAdditionalMetadata(OpfMetadata meta)
    {
        var additional = new Dictionary<string, JsonElement>(StringComparer.Ordinal);
        JsonElement S(string s) => JsonSerializer.SerializeToElement(s);

        if (meta.Identifier is not null) additional["identifier"] = S(meta.Identifier);
        if (meta.Publisher is not null) additional["publisher"] = S(meta.Publisher);
        if (meta.Subjects.Count > 0) additional["subject"] = S(meta.Subjects[0]);
        if (meta.Description is not null) additional["description"] = S(meta.Description);
        if (meta.Rights is not null) additional["rights"] = S(meta.Rights);
        return additional;
    }

    /// <summary>Namespace prefix every Dublin Core element carries.</summary>
    private const string DublinCoreNamespacePrefix = "http://purl.org/dc/elements/";

    /// <summary>
    /// Whether <paramref name="node"/> is package Dublin Core metadata: in the Dublin Core
    /// namespace and not inside an EPUB 3 <c>&lt;collection&gt;</c>, whose Dublin Core elements
    /// describe the collection rather than the book.
    /// </summary>
    private static bool IsDublinCore(XElement node) =>
        node.Name.NamespaceName.StartsWith(DublinCoreNamespacePrefix, StringComparison.Ordinal)
        && !node.Ancestors().Any(a => a.Name.LocalName.Equals("collection", StringComparison.OrdinalIgnoreCase));

    /// <summary>
    /// Pick the publication date from every <c>dc:date</c>. EPUB 2 qualifies dates with
    /// <c>opf:event</c>; a modification date is used only when no other date exists.
    /// </summary>
    private static string? SelectPublicationDate(List<(string? Event, string Value)> dates)
    {
        foreach (var (evt, value) in dates)
            if (evt is null or "publication" or "creation" or "original-publication")
                return value;
        return dates.Count > 0 ? dates[0].Value : null;
    }

    /// <summary>
    /// Mirrors Rust `node.text()`: only the element's direct text content, trimmed. <c>null</c>
    /// when the element has no text nodes of its own.
    /// </summary>
    private static string? DirectText(XElement node)
    {
        var sb = new StringBuilder();
        bool sawText = false;
        foreach (var child in node.Nodes())
        {
            if (child is XText t)
            {
                sb.Append(t.Value);
                sawText = true;
            }
        }
        return sawText ? sb.ToString().Trim() : null;
    }
}
