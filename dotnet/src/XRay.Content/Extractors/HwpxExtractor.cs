// Ported from Rust `crates/xberg/src/extractors/hwpx.rs` and the `unhwp` crate's HWPX path
// (container.rs / section.rs / styles.rs / mod.rs). Self-contained: parses the ZIP-based
// Hangul OWPML container directly with System.IO.Compression + System.Xml.Linq.

using System.IO.Compression;
using System.Text;
using System.Text.Json;
using System.Xml.Linq;
using XRay.Content.Core;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>Extractor for Hangul Word Processor XML (.hwpx) files.</summary>
public sealed class HwpxExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/haansofthwpx" };

    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        var limits = config.SecurityLimits ?? new SecurityLimits();
        // The declared file size is refused before the zip is even opened: a container that
        // large has nothing legitimate to say and reading its directory is work already done
        // on an attacker's behalf.
        if (content.Length > limits.MaxArchiveSize)
            throw new ValidationException(
                $"HWPX file exceeds size limit ({content.Length} > {limits.MaxArchiveSize} bytes)");

        using var stream = new MemoryStream(content.ToArray(), writable: false);
        ZipArchive archive;
        try
        {
            archive = ZipBombValidator.OpenValidated(stream, limits);
        }
        catch (XRay.Content.Core.SecurityException e)
        {
            throw new ValidationException(e.Message);
        }
        using var _archive = archive;

        var builder = new InternalDocumentBuilder("hwpx");
        builder.SetMimeType(mimeType);

        // Heading-level map from Contents/header.xml paragraph styles.
        var headingLevels = ParseStyles(ReadEntry(archive, "Contents/header.xml"));

        // Metadata from Contents/content.hpf (OPF).
        string? contentHpf = ReadEntry(archive, "Contents/content.hpf");
        var meta = ParseMetadata(contentHpf);

        // Section list (spine order, else scan for Contents/section*.xml).
        var sections = ListSections(archive, contentHpf);
        foreach (var path in sections)
        {
            string? xml = ReadEntry(archive, path);
            if (xml is null) continue;
            ProcessSection(Cancel.ParseXml(xml, LoadOptions.PreserveWhitespace).Root!, builder, headingLevels);
        }

        var doc = builder.Build();
        doc.Metadata = meta;
        return doc;
    }

    // ── metadata ────────────────────────────────────────────────────────────

    private static Metadata ParseMetadata(string? hpf)
    {
        var m = new Metadata { DocumentVersion = "HWPX" };
        if (hpf is null) return m;

        XDocument doc;
        try { doc = Cancel.ParseXml(hpf); }
        catch (Exception) when (Cancel.NotCancelling) { return m; }

        string? title = null, creator = null, subject = null, date = null, modified = null, generator = null;
        var keywords = new List<string>();

        foreach (var el in doc.Descendants())
        {
            string name = el.Name.LocalName;
            string val = el.Value.Trim();
            switch (name)
            {
                case "title": if (val.Length > 0) title = val; break;
                case "creator": if (val.Length > 0) creator = val; break;
                case "description": if (val.Length > 0) subject = val; break;
                case "date": if (val.Length > 0) date = val; break;
                case "modified": if (val.Length > 0) modified = val; break;
                case "generator": if (val.Length > 0) generator = val; break;
                case "subject":
                case "keywords":
                    foreach (var kw in val.Split(',', ';', '|'))
                    {
                        string t = kw.Trim();
                        if (t.Length > 0 && !keywords.Contains(t)) keywords.Add(t);
                    }
                    break;
            }
        }

        if (title is not null) m.Title = title;
        if (creator is not null) m.Authors = new List<string> { creator };
        if (subject is not null) m.Subject = subject;
        if (keywords.Count > 0) m.Keywords = keywords;
        if (date is not null) m.CreatedAt = date;
        if (modified is not null) m.ModifiedAt = modified;
        if (generator is not null)
            m.Additional["creator_app"] = JsonSerializer.SerializeToElement(generator);
        return m;
    }

    // ── styles (heading levels) ───────────────────────────────────────────────

    private static Dictionary<uint, byte> ParseStyles(string? headerXml)
    {
        var map = new Dictionary<uint, byte>();
        if (headerXml is null) return map;

        XDocument doc;
        try { doc = Cancel.ParseXml(headerXml); }
        catch (Exception) when (Cancel.NotCancelling) { return map; }

        foreach (var el in doc.Descendants())
        {
            string name = el.Name.LocalName;
            if (name is not ("paraShape" or "paraPr" or "paraProperties")) continue;
            uint id = GetUIntAttr(el, "id") ?? 0;
            byte level = 0;
            foreach (var child in el.Descendants())
            {
                string cn = child.Name.LocalName;
                if (cn is "outlineLevel" or "heading" or "level")
                {
                    int? lv = GetIntAttr(child, "val") ?? GetIntAttr(child, "level");
                    if (lv is > 0) level = (byte)Math.Min(lv.Value, 6);
                }
            }
            if (level > 0) map[id] = level;
        }
        return map;
    }

    // ── section walk ──────────────────────────────────────────────────────────

    private static void ProcessSection(XElement sec, InternalDocumentBuilder builder, Dictionary<uint, byte> headings)
    {
        foreach (var child in sec.Elements())
        {
            switch (child.Name.LocalName)
            {
                case "p": ProcessParagraph(child, builder, headings); break;
                case "tbl": ProcessTable(child, builder); break;
            }
        }
    }

    private static void ProcessParagraph(XElement p, InternalDocumentBuilder builder, Dictionary<uint, byte> headings)
    {
        byte headingLevel = ResolveHeadingLevel(p, headings);
        string text = ParagraphText(p).Trim();

        if (text.Length > 0)
        {
            if (headingLevel > 0)
                builder.PushHeading(headingLevel, text, null, null);
            else
                builder.PushParagraph(text, new(), null, null);
        }

        // Tables nested inside runs / ctrl appear as separate blocks after the paragraph.
        foreach (var tbl in p.Descendants().Where(e => e.Name.LocalName == "tbl"))
            ProcessTable(tbl, builder);
    }

    private static byte ResolveHeadingLevel(XElement p, Dictionary<uint, byte> headings)
    {
        byte level = 0;
        uint? paraPr = GetUIntAttr(p, "paraPrIDRef");
        if (paraPr is not null && headings.TryGetValue(paraPr.Value, out var l1)) level = l1;
        uint? styleId = GetUIntAttr(p, "styleIDRef");
        if (styleId is not null && headings.TryGetValue(styleId.Value, out var l2) && l2 > 0) level = l2;
        return level;
    }

    /// <summary>Concatenate `hp:t` text within a paragraph's runs, skipping content under
    /// `ctrl` elements and emitting a space for each `hp:tab`. Nested table cell text is
    /// excluded (handled separately).</summary>
    private static string ParagraphText(XElement p)
    {
        var sb = new StringBuilder();
        foreach (var run in p.Elements().Where(e => e.Name.LocalName == "run"))
            AppendRunText(run, sb);
        return sb.ToString();
    }

    private static void AppendRunText(XElement node, StringBuilder sb)
    {
        foreach (var child in node.Elements())
        {
            string name = child.Name.LocalName;
            if (name == "ctrl") continue;         // skip control fields (formulas, footnotes, …)
            if (name == "tbl") continue;           // tables handled separately
            if (name == "t") { sb.Append(child.Value); continue; }
            if (name == "tab") { sb.Append(' '); continue; }
            AppendRunText(child, sb);
        }
    }

    private static void ProcessTable(XElement tbl, InternalDocumentBuilder builder)
    {
        var rows = new List<List<string>>();
        foreach (var tr in tbl.Elements().Where(e => e.Name.LocalName == "tr"))
        {
            var cells = new List<string>();
            foreach (var tc in tr.Elements().Where(e => e.Name.LocalName == "tc"))
            {
                var sb = new StringBuilder();
                foreach (var cp in tc.Descendants().Where(e => e.Name.LocalName == "p"))
                    sb.Append(ParagraphText(cp));
                cells.Add(sb.ToString());
            }
            rows.Add(cells);
        }
        if (rows.Count > 0)
            builder.PushTableFromCells(rows, null, null);
    }

    // ── section listing ──────────────────────────────────────────────────────

    private static List<string> ListSections(ZipArchive archive, string? hpf)
    {
        var sections = new List<string>();

        if (hpf is not null)
        {
            try
            {
                var doc = Cancel.ParseXml(hpf);
                var manifest = new Dictionary<string, string>(StringComparer.Ordinal);
                var spine = new List<string>();
                foreach (var el in doc.Descendants())
                {
                    switch (el.Name.LocalName)
                    {
                        case "item":
                            string? id = el.Attribute("id")?.Value;
                            string? href = el.Attribute("href")?.Value;
                            if (id is not null && href is not null) manifest[id] = href;
                            break;
                        case "itemref":
                            string? idref = el.Attribute("idref")?.Value;
                            if (idref is not null) spine.Add(idref);
                            break;
                    }
                }
                foreach (var idref in spine)
                    if (manifest.TryGetValue(idref, out var href)
                        && href.EndsWith(".xml", StringComparison.OrdinalIgnoreCase)
                        && href.ToLowerInvariant().Contains("section"))
                        sections.Add(href);
            }
            catch (Exception) when (Cancel.NotCancelling) { /* fall through to scan */ }
        }

        if (sections.Count == 0)
        {
            foreach (var entry in archive.Entries)
                if (entry.FullName.StartsWith("Contents/section", StringComparison.Ordinal)
                    && entry.FullName.EndsWith(".xml", StringComparison.Ordinal))
                    sections.Add(entry.FullName);
            sections.Sort(StringComparer.Ordinal);
        }

        if (sections.Count == 0)
            throw new InvalidDataException("HWPX archive has no section files");
        return sections;
    }

    // ── helpers ────────────────────────────────────────────────────────────────

    private static string? ReadEntry(ZipArchive archive, string path)
    {
        var entry = archive.GetEntry(path);
        if (entry is null) return null;
        using var s = entry.Open();
        using var r = new StreamReader(s, Encoding.UTF8);
        return r.ReadToEnd();
    }

    private static uint? GetUIntAttr(XElement e, string name) =>
        uint.TryParse(e.Attribute(name)?.Value, out var v) ? v : null;

    private static int? GetIntAttr(XElement e, string name) =>
        int.TryParse(e.Attribute(name)?.Value, out var v) ? v : null;
}
