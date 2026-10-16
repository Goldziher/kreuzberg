// Link targets: a /Dest or a /A action (URI, GoTo) resolved to an ExtractedUri the way
// crates/xberg/src/pdf/bookmarks.rs does for outline items. Shared by the outline walker and
// the annotation reader, so a link annotation and a bookmark to the same place agree.
using XRay.Content.Types;

namespace XRay.Content.Internal.Pdf;

internal sealed class PdfLinkResolver
{
    private readonly PdfDocument _doc;
    private readonly Dictionary<string, PdfObject> _named;
    private readonly Dictionary<PdfRef, int> _pageNumbers;

    public PdfLinkResolver(PdfDocument doc, Dictionary<string, PdfObject> named, Dictionary<PdfRef, int> pageNumbers)
    {
        _doc = doc;
        _named = named;
        _pageNumbers = pageNumbers;
    }

    public static PdfLinkResolver For(PdfDocument doc) =>
        new(doc,
            doc.Catalog is { } catalog ? PdfBookmarks.CollectNamedDestinations(doc, catalog) : new(StringComparer.Ordinal),
            doc.PageNumbersByRef);

    /// <summary>An internal destination as an anchor: <c>#name</c> for a named one, else <c>#page=N</c>.</summary>
    public ExtractedUri? DestinationUri(PdfObject? destination, string? label)
    {
        if (destination is null) return null;
        var resolved = _doc.Resolve(destination);
        string? name = resolved switch
        {
            PdfString s => PdfBookmarks.Latin1(s.Bytes),
            PdfName n => n.Value,
            _ => null,
        };
        if (name is not null)
        {
            int? namedPage = _named.TryGetValue(name, out var target)
                ? PdfBookmarks.ResolveDestinationObject(
                    _doc, target, _named, _pageNumbers, new HashSet<string>(StringComparer.Ordinal) { name }, 0)
                : null;
            return Anchor("#" + name, label, namedPage);
        }

        // An array (or a dictionary holding one): its page is the first element, which has to
        // be read as a reference before anything dereferences it.
        if (resolved is not (PdfArray or PdfDict)) return null;
        int? page = PdfBookmarks.ResolveDestinationObject(
            _doc, destination, _named, _pageNumbers, new HashSet<string>(StringComparer.Ordinal), 0);
        return Anchor($"#page={page ?? 1}", label, page);
    }

    /// <summary>A <c>/URI</c> action as a hyperlink, a <c>/GoTo</c> action as an anchor.</summary>
    public ExtractedUri? ActionUri(PdfDict? action, string? label)
    {
        if (action is null) return null;
        switch (_doc.Resolve(action.Get("S")).AsName())
        {
            case "URI":
                if (UriText(action) is { } url)
                    return new ExtractedUri { Url = url, Label = label, Kind = UriKind.Hyperlink };
                return null;
            case "GoTo":
                return DestinationUri(action.Get("D"), label);
            default:
                return null;
        }
    }

    /// <summary>The action's <c>/URI</c> string, lossily decoded as UTF-8 as upstream does.</summary>
    public string? UriText(PdfDict action) =>
        _doc.Resolve(action.Get("URI")).AsStringBytes() is { } bytes
            ? System.Text.Encoding.UTF8.GetString(bytes)
            : null;

    private static ExtractedUri Anchor(string url, string? label, int? page) =>
        new() { Url = url, Label = label, Kind = UriKind.Anchor, Page = page is { } p ? (uint)p : null };
}
