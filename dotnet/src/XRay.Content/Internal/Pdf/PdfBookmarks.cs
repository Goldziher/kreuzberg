// Ported from crates/xberg/src/pdf/bookmarks.rs.
//
// Walks the document outline (bookmarks) from the catalog and resolves each
// item's destination to a page number, so the structure pipeline can recover
// headings the font-size classifier could not see.
using System;
using System.Collections.Generic;

namespace XRay.Content.Internal.Pdf;

/// <summary>A resolved PDF outline item retained for structural recovery.</summary>
public sealed class PdfOutlineEntry
{
    /// <summary>The human-readable outline title.</summary>
    public string Title = "";

    /// <summary>Zero-based depth relative to the document's root outline items.</summary>
    public int Depth;

    /// <summary>One-based destination page, when the destination resolves within this PDF.</summary>
    public int? PageNumber;

    /// <summary>
    /// The entry as a link: an external URL (<c>/A /S /URI</c>) as a hyperlink, an internal
    /// destination as an anchor (<c>#name</c> or <c>#page=N</c>) carrying its page. Null when the
    /// item has neither.
    /// </summary>
    public Types.ExtractedUri? Uri;
}

internal static class PdfBookmarks
{
    private const int MaxOutlineDepth = 50;
    private const int MaxOutlineItems = 500;
    private const int MaxNameTreeDepth = 50;
    private const int MaxNameTreeNodes = 500;
    private const int MaxNamedDestinations = 2000;
    private const int MaxDestinationHops = 50;

    /// <summary>
    /// Outline entries in document order. Traversal is bounded independently of
    /// successfully decoded entries, and each indirect object is visited at most once, so
    /// a malformed outline cycle cannot recurse indefinitely.
    /// </summary>
    public static List<PdfOutlineEntry> ExtractOutlineEntries(PdfDocument doc)
    {
        var entries = new List<PdfOutlineEntry>();
        var catalog = doc.Catalog;
        if (catalog is null) return entries;
        var outlines = doc.Resolve(catalog.Get("Outlines")).AsDict();
        if (outlines?.Get("First") is not PdfRef first) return entries;

        var named = CollectNamedDestinations(doc, catalog);
        var pageNumbers = doc.PageNumbersByRef;
        var visited = new HashSet<PdfRef>();
        Walk(doc, first, 0, named, pageNumbers, visited, entries);
        return entries;
    }

    /// <summary>The URIs the outline carries, in document order (upstream's bookmark URIs).</summary>
    public static List<Types.ExtractedUri> ExtractBookmarkUris(IEnumerable<PdfOutlineEntry> entries)
    {
        var uris = new List<Types.ExtractedUri>();
        foreach (var e in entries) if (e.Uri is not null) uris.Add(e.Uri);
        return uris;
    }

    private static void Walk(
        PdfDocument doc, PdfRef itemRef, int depth,
        Dictionary<string, PdfObject> named, Dictionary<PdfRef, int> pageNumbers,
        HashSet<PdfRef> visited, List<PdfOutlineEntry> entries)
    {
        if (depth > MaxOutlineDepth || visited.Count >= MaxOutlineItems || !visited.Add(itemRef)) return;

        var dict = doc.Resolve(itemRef).AsDict();
        if (dict is null) return;

        var entry = ExtractEntry(doc, dict, depth, named, pageNumbers);
        if (entry is not null) entries.Add(entry);

        if (dict.Get("First") is PdfRef child) Walk(doc, child, depth + 1, named, pageNumbers, visited, entries);
        if (dict.Get("Next") is PdfRef sibling) Walk(doc, sibling, depth, named, pageNumbers, visited, entries);
    }

    private static PdfOutlineEntry? ExtractEntry(
        PdfDocument doc, PdfDict dict, int depth,
        Dictionary<string, PdfObject> named, Dictionary<PdfRef, int> pageNumbers)
    {
        string? title = null;
        if (doc.Resolve(dict.Get("Title")).AsStringBytes() is { } bytes)
            title = PdfMetadataExtractor.DecodePdfString(bytes);

        int? pageNumber = ResolveDestination(doc, dict, named, pageNumbers);

        // The link form of the same item: /Dest first, then the action, as upstream does.
        var links = new PdfLinkResolver(doc, named, pageNumbers);
        var uri = links.DestinationUri(dict.Get("Dest"), title)
                  ?? links.ActionUri(doc.Resolve(dict.Get("A")).AsDict(), title);
        if (title is null && pageNumber is null) return null;

        return new PdfOutlineEntry { Title = title ?? "", Depth = depth, PageNumber = pageNumber, Uri = uri };
    }

    /// <summary>The item's page, from its /Dest or from a GoTo action's /D.</summary>
    private static int? ResolveDestination(
        PdfDocument doc, PdfDict dict,
        Dictionary<string, PdfObject> named, Dictionary<PdfRef, int> pageNumbers)
    {
        if (dict.Get("Dest") is { } dest)
        {
            int? page = ResolveDestinationObject(doc, dest, named, pageNumbers, new HashSet<string>(StringComparer.Ordinal), 0);
            if (page is not null) return page;
        }

        var action = doc.Resolve(dict.Get("A")).AsDict();
        if (action is null) return null;
        if (doc.Resolve(action.Get("S")).AsName() != "GoTo") return null;
        if (action.Get("D") is not { } d) return null;
        return ResolveDestinationObject(doc, d, named, pageNumbers, new HashSet<string>(StringComparer.Ordinal), 0);
    }

    internal static int? ResolveDestinationObject(
        PdfDocument doc, PdfObject destination,
        Dictionary<string, PdfObject> named, Dictionary<PdfRef, int> pageNumbers,
        HashSet<string> visitedNames, int hops)
    {
        if (hops > MaxDestinationHops) return null;

        // The page reference has to survive resolution, so read the array before
        // dereferencing anything inside it.
        if (destination is PdfRef && doc.Resolve(destination) is PdfArray resolvedArray)
            return PageOfDestinationArray(resolvedArray, pageNumbers);

        switch (destination)
        {
            case PdfArray array:
                return PageOfDestinationArray(array, pageNumbers);
            case PdfDict dict when dict.Get("D") is { } inner:
                return ResolveDestinationObject(doc, inner, named, pageNumbers, visitedNames, hops + 1);
            case PdfString s:
            {
                string name = Latin1(s.Bytes);
                if (!visitedNames.Add(name)) return null;
                return named.TryGetValue(name, out var target)
                    ? ResolveDestinationObject(doc, target, named, pageNumbers, visitedNames, hops + 1)
                    : null;
            }
            case PdfName n:
            {
                if (!visitedNames.Add(n.Value)) return null;
                return named.TryGetValue(n.Value, out var target)
                    ? ResolveDestinationObject(doc, target, named, pageNumbers, visitedNames, hops + 1)
                    : null;
            }
            default:
            {
                var asDict = doc.Resolve(destination).AsDict();
                if (asDict?.Get("D") is { } innerD)
                    return ResolveDestinationObject(doc, innerD, named, pageNumbers, visitedNames, hops + 1);
                return null;
            }
        }
    }

    private static int? PageOfDestinationArray(PdfArray array, Dictionary<PdfRef, int> pageNumbers)
    {
        if (array.Items.Count == 0) return null;
        return array.Items[0] is PdfRef pageRef && pageNumbers.TryGetValue(pageRef, out int number) ? number : null;
    }

    internal static Dictionary<string, PdfObject> CollectNamedDestinations(PdfDocument doc, PdfDict catalog)
    {
        var destinations = new Dictionary<string, PdfObject>(StringComparer.Ordinal);

        // The pre-1.2 form: /Dests is a plain name → destination dictionary.
        if (doc.Resolve(catalog.Get("Dests")).AsDict() is { } legacy)
            foreach (var kv in legacy.Map)
            {
                if (destinations.Count >= MaxNamedDestinations) break;
                destinations[kv.Key] = kv.Value;
            }

        var names = doc.Resolve(catalog.Get("Names")).AsDict();
        if (names?.Get("Dests") is { } dests)
            CollectNameTree(doc, dests, destinations, new NameTreeBudget(), 0);

        return destinations;
    }

    private sealed class NameTreeBudget
    {
        public readonly HashSet<PdfRef> Visited = new();
        public int AttemptedNodes;
    }

    private static void CollectNameTree(
        PdfDocument doc, PdfObject obj, Dictionary<string, PdfObject> destinations, NameTreeBudget budget, int depth)
    {
        if (depth > MaxNameTreeDepth || budget.AttemptedNodes >= MaxNameTreeNodes) return;
        budget.AttemptedNodes++;
        if (obj is PdfRef r && !budget.Visited.Add(r)) return;

        var dict = doc.Resolve(obj).AsDict();
        if (dict is null) return;

        if (doc.Resolve(dict.Get("Names")).AsArray() is { } pairs)
            for (int i = 0; i + 1 < pairs.Items.Count; i += 2)
            {
                if (destinations.Count >= MaxNamedDestinations) break;
                if (pairs.Items[i] is PdfString key) destinations[Latin1(key.Bytes)] = pairs.Items[i + 1];
                else if (pairs.Items[i] is PdfName nameKey) destinations[nameKey.Value] = pairs.Items[i + 1];
            }

        var kids = doc.Resolve(dict.Get("Kids")).AsArray();
        if (kids is null) return;
        foreach (var kid in kids.Items)
        {
            if (budget.AttemptedNodes >= MaxNameTreeNodes) break;
            CollectNameTree(doc, kid, destinations, budget, depth + 1);
        }
    }

    /// <summary>Destination names are byte strings; compare them as bytes, not as text.</summary>
    internal static string Latin1(byte[] bytes)
    {
        var chars = new char[bytes.Length];
        for (int i = 0; i < bytes.Length; i++) chars[i] = (char)bytes[i];
        return new string(chars);
    }
}
