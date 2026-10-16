// Ported from Rust `crates/xberg/src/extraction/office_metadata/odt_properties.rs`
// and the shared helpers in `office_metadata/mod.rs` (parse_xml_text / parse_xml_int).

using System.IO.Compression;
using System.Xml.Linq;

namespace XRay.Content.Internal.Odf;

/// <summary>OpenDocument metadata from meta.xml. Mirrors Rust `OdtProperties`.</summary>
internal sealed class OdtProperties
{
    public string? Title;
    public string? Subject;
    public string? Creator;
    public string? InitialCreator;
    public string? Keywords;
    public string? Description;
    public string? Date;
    public string? CreationDate;
    public string? Language;
    public string? Generator;
    public string? EditingDuration;
    public string? EditingCycles;
    public int? PageCount;
    public int? WordCount;
    public int? CharacterCount;
    public int? ParagraphCount;
    public int? TableCount;
    public int? ImageCount;
}

/// <summary>Parses ODT metadata from meta.xml. Ports Rust `extract_odt_properties`.</summary>
internal static class OdfMetadata
{
    public static OdtProperties Extract(ZipArchive archive)
    {
        var xml = ReadEntry(archive, "meta.xml");
        if (xml is null)
            return new OdtProperties();

        // Rust returns Err on malformed XML; XDocument.Parse throws, which the caller lets propagate.
        var doc = Cancel.ParseXml(xml, LoadOptions.PreserveWhitespace);
        var root = doc.Root!;

        return new OdtProperties
        {
            Title = ParseXmlText(root, "title"),
            Subject = ParseXmlText(root, "subject"),
            Creator = ParseXmlText(root, "creator"),
            Description = ParseXmlText(root, "description"),
            Language = ParseXmlText(root, "language"),
            Date = ParseXmlText(root, "date"),
            InitialCreator = ParseXmlText(root, "initial-creator"),
            Keywords = ParseXmlText(root, "keyword"),
            CreationDate = ParseXmlText(root, "creation-date"),
            Generator = ParseXmlText(root, "generator"),
            EditingDuration = ParseXmlText(root, "editing-duration"),
            EditingCycles = ParseXmlText(root, "editing-cycles"),
            PageCount = ParseXmlInt(root, "page-count"),
            WordCount = ParseXmlInt(root, "word-count"),
            CharacterCount = ParseXmlInt(root, "character-count"),
            ParagraphCount = ParseXmlInt(root, "paragraph-count"),
            TableCount = ParseXmlInt(root, "table-count"),
            ImageCount = ParseXmlInt(root, "image-count"),
        };
    }

    private static string? ReadEntry(ZipArchive archive, string name)
    {
        var entry = archive.GetEntry(name);
        if (entry is null)
            return null;
        using var stream = entry.Open();
        using var reader = new StreamReader(stream);
        return Cancel.ReadToEnd(reader);
    }

    /// <summary>
    /// Mirrors Rust `parse_xml_text`: first descendant with the given local name, its direct text,
    /// trimmed, dropped if empty.
    /// </summary>
    private static string? ParseXmlText(XElement root, string name)
    {
        var node = FindByLocalName(root, name);
        if (node is null)
            return null;
        var text = OdfContentParser.NodeText(node);
        if (text is null)
            return null;
        text = text.Trim();
        return text.Length == 0 ? null : text;
    }

    /// <summary>Mirrors Rust `parse_xml_int`: first descendant with the given local name parsed as i32.</summary>
    private static int? ParseXmlInt(XElement root, string name)
    {
        var node = FindByLocalName(root, name);
        var text = node is null ? null : OdfContentParser.NodeText(node);
        if (text is null)
            return null;
        return int.TryParse(text.Trim(), out var v) ? v : null;
    }

    // roxmltree `descendants()` includes the node itself, in document (preorder) order.
    private static XElement? FindByLocalName(XElement root, string name)
    {
        if (root.Name.LocalName == name)
            return root;
        foreach (var d in root.Descendants())
            if (d.Name.LocalName == name)
                return d;
        return null;
    }
}
