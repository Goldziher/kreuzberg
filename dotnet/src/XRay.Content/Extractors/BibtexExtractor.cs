using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.Bibtex;
using XRay.Content.Internal.Markup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// BibTeX bibliography extractor. Ported from Rust `extractors/bibtex.rs` (which uses the
/// `biblatex` crate). Each entry becomes a Citation element whose text is the re-formatted entry
/// with fields sorted alphabetically. The C# <see cref="BibtexMetadata"/> payload is a stub
/// (entry_count only); citation_keys/year_range/entry_types are not represented (documented gap).
/// </summary>
public sealed class BibtexExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/x-bibtex", "text/x-bibtex", "application/x-biblatex" };
    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        string bibtex = Encoding.UTF8.GetString(content);
        var builder = new InternalDocumentBuilder("bibtex");

        var entries = BibtexParser.Parse(bibtex);
        var authorsSet = new SortedSet<string>(StringComparer.Ordinal);
        var entriesMeta = new List<Dictionary<string, string>>();
        var citationKeys = new List<string>();
        var yearsSet = new SortedSet<uint>();
        var entryTypes = new SortedDictionary<string, long>(StringComparer.Ordinal);

        foreach (var entry in entries)
        {
            Cancel.ThrowIfRequested();
            citationKeys.Add(entry.Key);
            entryTypes[entry.EntryType] = entryTypes.GetValueOrDefault(entry.EntryType) + 1;
            var sb = new StringBuilder();
            sb.Append('@').Append(entry.EntryType).Append('{').Append(entry.Key).Append(",\n");
            var entryFields = new Dictionary<string, string> { ["entry_type"] = entry.EntryType };
            foreach (var (name, value) in entry.Fields)
            {
                sb.Append("  ").Append(name).Append(" = {").Append(value).Append("},\n");
                entryFields[name] = value;
                if (name == "author")
                    foreach (var a in value.Split(" and ")) { string ta = a.Trim(); if (ta.Length != 0) authorsSet.Add(ta); }
                if (name == "year" && uint.TryParse(value, out var yr)) yearsSet.Add(yr);
            }
            sb.Append("}\n\n");

            string citationText = sb.ToString().Trim();

            string linkLabel = entryFields.TryGetValue("title", out var t) && t.Length != 0 ? t : entry.Key;
            if (entryFields.TryGetValue("url", out var url) && url.Length != 0)
                builder.PushUri(MarkupHelpers.Hyperlink(url, linkLabel));
            if (entryFields.TryGetValue("doi", out var doi) && doi.Length != 0)
                builder.PushUri(MarkupHelpers.Citation($"https://doi.org/{doi}", linkLabel));

            uint idx = builder.PushCitation(citationText, entry.Key, null);

            var linkAnns = new List<TextAnnotation>();
            uint textLen = (uint)Encoding.UTF8.GetByteCount(citationText);
            if (entryFields.TryGetValue("url", out var url2) && url2.Length != 0)
                linkAnns.Add(MarkupHelpers.Annotation(0, textLen, MarkupHelpers.Link(url2, linkLabel)));
            if (entryFields.TryGetValue("doi", out var doi2) && doi2.Length != 0)
            {
                string doiUrl = doi2.StartsWith("http") ? doi2 : $"https://doi.org/{doi2}";
                linkAnns.Add(MarkupHelpers.Annotation(0, textLen, MarkupHelpers.Link(doiUrl, linkLabel)));
            }
            if (linkAnns.Count > 0) builder.SetAnnotations(idx, linkAnns);
            builder.SetAttributes(idx, entryFields);

            var entryObj = new Dictionary<string, string> { ["key"] = entry.Key };
            foreach (var kv in entryFields) entryObj[kv.Key] = kv.Value;
            entriesMeta.Add(entryObj);
        }

        var doc = builder.Build();
        doc.MimeType = mimeType;

        var authorsList = authorsSet.ToList();
        var bibMeta = new BibtexMetadata
        {
            EntryCount = entries.Count,
            CitationKeys = citationKeys,
            Authors = authorsList,
            YearRange = yearsSet.Count > 0
                ? new YearRange { Min = yearsSet.Min, Max = yearsSet.Max, Years = yearsSet.ToList() }
                : null,
            EntryTypes = entryTypes.Count > 0 ? entryTypes : null,
        };
        var meta = new Metadata
        {
            Format = new FormatMetadata { FormatType = "bibtex", Payload = bibMeta },
        };
        if (authorsList.Count > 0) meta.Authors = authorsList;
        meta.Additional["entries"] = JsonSerializer.SerializeToElement(entriesMeta, Json.Options);
        doc.Metadata = meta;
        return doc;
    }
}
