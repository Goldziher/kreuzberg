using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Xml;
using System.Xml.Linq;
using XRay.Content.Core;
using XRay.Content.Internal.Markup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// OPML (Outline Processor Markup Language) extractor. Ported from Rust `extractors/opml/`.
/// Maps the outline hierarchy to headings and derives head metadata + feed URLs.
/// </summary>
public sealed partial class OpmlExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "text/x-opml", "application/xml+opml", "application/x-opml+xml" };
    public int Priority => 55;

    [GeneratedRegex(@"<(?:strong|b)>(.*?)</(?:strong|b)>")] private static partial Regex StrongRe();
    [GeneratedRegex(@"<(?:em|i)>(.*?)</(?:em|i)>")] private static partial Regex EmRe();
    [GeneratedRegex(@"<a\s+href=""([^""]*)""[^>]*>(.*?)</a>")] private static partial Regex LinkRe();

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        string xml = DecodeText(content);
        XDocument xdoc = Cancel.ParseXml(xml, LoadOptions.PreserveWhitespace);

        var meta = new Metadata();
        var additional = new Dictionary<string, object>();

        var opml = xdoc.Root is { } r && r.Name.LocalName == "opml" ? r
            : xdoc.Descendants().FirstOrDefault(e => e.Name.LocalName == "opml");

        var builder = new InternalDocumentBuilder("opml");

        if (opml is not null)
        {
            var head = opml.Elements().FirstOrDefault(e => e.Name.LocalName == "head");
            if (head is not null)
            {
                foreach (var child in head.Elements())
                {
                    string tag = child.Name.LocalName;
                    string text = (child.Value ?? "").Trim();
                    if (text.Length == 0) continue;
                    switch (tag)
                    {
                        case "title": meta.Title = text; break;
                        case "dateCreated": meta.CreatedAt = text; break;
                        case "dateModified": meta.ModifiedAt = text; break;
                        case "ownerName": meta.CreatedBy = text; break;
                        case "ownerEmail": additional["ownerEmail"] = text; break;
                    }
                }
            }

            var body = opml.Elements().FirstOrDefault(e => e.Name.LocalName == "body");
            if (body is not null)
            {
                var feedUrls = new List<Dictionary<string, string>>();
                foreach (var outline in body.Elements().Where(e => e.Name.LocalName == "outline"))
                {
                    BuildOutline(outline, 1, builder);
                    CollectFeedUrls(outline, feedUrls);
                }
                if (feedUrls.Count > 0) additional["feed_urls"] = feedUrls;
            }
        }

        foreach (var (k, v) in additional) meta.Additional[k] = JsonSerializer.SerializeToElement(v, Json.Options);

        var doc = builder.Build();
        doc.MimeType = mimeType;
        doc.Metadata = meta;
        return doc;
    }

    private static void BuildOutline(XElement node, byte depth, InternalDocumentBuilder builder)
    {
        string text = (node.Attribute("text")?.Value ?? "").Trim();
        var childOutlines = node.Elements().Where(e => e.Name.LocalName == "outline").ToList();

        if (text.Length == 0)
        {
            // An outline with no `text` still carries content when it has a `_note`; only a
            // truly empty node contributes nothing. Without this the note is the one thing on
            // the node and it goes unread.
            string? emptyNodeNote = node.Attribute("_note")?.Value.Trim();
            if (!string.IsNullOrEmpty(emptyNodeNote))
                builder.PushParagraph(ConvertInlineHtml(emptyNodeNote), new List<XRay.Content.Types.TextAnnotation>(), null, null);
            foreach (var c in childOutlines) BuildOutline(c, depth, builder);
            return;
        }

        var attrs = ExtractOutlineAttributes(node);
        string? label = text.Length == 0 ? null : text;
        string? xmlUrl = node.Attribute("xmlUrl")?.Value.Trim();
        if (!string.IsNullOrEmpty(xmlUrl)) builder.PushUri(MarkupHelpers.Hyperlink(xmlUrl, label));
        string? htmlUrl = node.Attribute("htmlUrl")?.Value.Trim();
        if (!string.IsNullOrEmpty(htmlUrl)) builder.PushUri(MarkupHelpers.Hyperlink(htmlUrl, label));

        byte level = Math.Min(depth, (byte)6);
        string converted = ConvertInlineHtml(text);
        uint idx = builder.PushHeading(level, converted, null, null);
        if (attrs.Count > 0) builder.SetAttributes(idx, attrs);
        foreach (var c in childOutlines) BuildOutline(c, (byte)(depth + 1), builder);
    }

    private static Dictionary<string, string> ExtractOutlineAttributes(XElement node)
    {
        var attrs = new Dictionary<string, string>();
        foreach (var name in new[] { "xmlUrl", "htmlUrl", "type", "description", "_note" })
        {
            string? val = node.Attribute(name)?.Value.Trim();
            if (!string.IsNullOrEmpty(val)) attrs[name] = val;
        }
        return attrs;
    }

    private static void CollectFeedUrls(XElement node, List<Dictionary<string, string>> urls)
    {
        string? xmlUrl = node.Attribute("xmlUrl")?.Value.Trim();
        if (!string.IsNullOrEmpty(xmlUrl))
        {
            var entry = new Dictionary<string, string> { ["xmlUrl"] = xmlUrl };
            string? text = node.Attribute("text")?.Value.Trim();
            if (!string.IsNullOrEmpty(text)) entry["text"] = text;
            string? htmlUrl = node.Attribute("htmlUrl")?.Value.Trim();
            if (!string.IsNullOrEmpty(htmlUrl)) entry["htmlUrl"] = htmlUrl;
            string? feedType = node.Attribute("type")?.Value.Trim();
            if (!string.IsNullOrEmpty(feedType)) entry["type"] = feedType;
            urls.Add(entry);
        }
        foreach (var child in node.Elements().Where(e => e.Name.LocalName == "outline"))
            CollectFeedUrls(child, urls);
    }

    private static string ConvertInlineHtml(string text)
    {
        string result = text;
        result = StrongRe().Replace(result, "**$1**");
        result = EmRe().Replace(result, "*$1*");
        result = LinkRe().Replace(result, "[$2]($1)");
        result = result.Replace("\\<", "<").Replace("\\>", ">");
        return result;
    }

    private static string DecodeText(ReadOnlySpan<byte> content)
    {
        if (content.Length >= 3 && content[0] == 0xEF && content[1] == 0xBB && content[2] == 0xBF)
            return Encoding.UTF8.GetString(content[3..]);
        return Encoding.UTF8.GetString(content);
    }
}
