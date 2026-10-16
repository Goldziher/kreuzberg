// Ported from crates/xberg/src/extractors/odp.rs (`build_internal_document`,
// `process_page_object`, `extract_odp_notes_text`, `extract_odp_master_page_text`).

using System.IO.Compression;
using System.Xml.Linq;
using XRay.Content.Types;

using XRay.Content.Core;

namespace XRay.Content.Internal.Odf;

/// <summary>
/// Reads an OpenDocument Presentation's <c>content.xml</c> into an
/// <see cref="InternalDocument"/>, one slide marker per <c>draw:page</c>.
/// </summary>
/// <remarks>
/// A presentation shares ODT's ZIP-and-XML container, but its body is
/// <c>office:presentation &gt; draw:page</c> rather than <c>office:text</c>, and a slide's text
/// lives inside drawing frames. Only that outer traversal is specific to presentations: each text
/// box is handed to the ODT walker, which already understands paragraphs, headings, lists, nested
/// tables and inline images.
/// </remarks>
internal static class OdfPresentationParser
{
    public static InternalDocument BuildInternalDocument(ZipArchive archive, SecurityLimits? limits = null)
    {
        var budget = new SecurityBudget(limits ?? new SecurityLimits());
        var imageData = OdfContentParser.PreExtractImages(archive);
        var formulaData = OdfContentParser.PreExtractFormulas(archive);

        var contentXml = OdfContentParser.ReadEntry(archive, "content.xml");
        if (contentXml is null)
            return new InternalDocumentBuilder("odp").Build();

        var doc = Cancel.ParseXml(contentXml, LoadOptions.PreserveWhitespace);
        var root = doc.Root!;
        var styleMap = OdfStyles.BuildStyleMap(root);
        var listStyleMap = OdfStyles.BuildListStyleMap(root);
        var builder = new InternalDocumentBuilder("odp");

        // Reject a presentation whose slide count exceeds the configured ceiling before the
        // recursive per-slide walk of frames, shapes, tables and images below. Unlike PPTX, an
        // ODP's slide count is not available before parsing content.xml — ODF has no manifest of
        // per-slide archive entries — so it is counted off the DOM already parsed above.
        DocumentLimits.EnforcePageCount(
            root.Elements().Where(n => n.Name.LocalName == "body")
                .SelectMany(body => body.Elements().Where(n => n.Name.LocalName == "presentation"))
                .SelectMany(pres => pres.Elements().Where(n => n.Name.LocalName == "page"))
                .Count(),
            limits);

        uint slideNumber = 0;
        foreach (var body in root.Elements().Where(n => n.Name.LocalName == "body"))
        foreach (var presentation in body.Elements().Where(n => n.Name.LocalName == "presentation"))
        foreach (var page in presentation.Elements().Where(n => n.Name.LocalName == "page"))
        {
            budget.Step();
            slideNumber++;
            builder.PushSlide(slideNumber, OdfStyles.Attr(page, "name"), null);

            // Direct children only, never descendants: the slide's speaker notes are a sibling of
            // its drawing frames, and walking descendants would fold them into the slide's own
            // text. Groups are recursed into below, so nothing on the slide is missed.
            foreach (var pageChild in page.Elements())
                if (pageChild.Name.LocalName is "frame" or "g")
                    ProcessPageObject(pageChild, builder, styleMap, listStyleMap, imageData, formulaData, budget);

            var notes = page.Elements().FirstOrDefault(n => n.Name.LocalName == "notes");
            if (notes is not null && ExtractNotesText(notes) is { } notesText)
                builder.PushRawBlock("odp-speaker-notes", notesText, slideNumber);
        }

        ExtractMasterPageText(archive, builder);
        return builder.Build();
    }

    /// <summary>
    /// Handle one drawing object on a slide: a frame or group to recurse into, a shape whose text
    /// the ODT walker reads, a table, an image, or an embedded object.
    /// </summary>
    /// <remarks>
    /// Shapes other than a frame's text box — custom shapes, rectangles, ellipses, connectors —
    /// carry their own paragraphs, and a group can nest frames and shapes to any depth, so every
    /// shape kind is named here rather than assuming text only ever sits in a text box.
    /// </remarks>
    private static void ProcessPageObject(
        XElement node,
        InternalDocumentBuilder builder,
        Dictionary<string, OdtStyleProps> styleMap,
        Dictionary<string, bool> listStyleMap,
        Dictionary<string, OdfImage> imageData,
        Dictionary<string, string> formulaData,
        SecurityBudget budget)
    {
        switch (node.Name.LocalName)
        {
            case "frame":
            case "g":
                foreach (var child in node.Elements())
                    ProcessPageObject(child, builder, styleMap, listStyleMap, imageData, formulaData, budget);
                break;

            case "text-box":
            case "custom-shape":
            case "rect":
            case "ellipse":
            case "circle":
            case "line":
            case "polygon":
            case "polyline":
            case "path":
            case "connector":
            case "regular-polygon":
            case "measure":
                OdfContentParser.BuildInternalElements(node, builder, styleMap, listStyleMap, imageData, formulaData, budget);
                break;

            case "table":
            {
                var cells = OdfContentParser.ExtractTableCells(node);
                if (cells.Count > 0)
                {
                    long cellCount = 0;
                    foreach (var row in cells) cellCount += row.Count;
                    budget.AddCells(cellCount);
                    builder.PushTableFromCells(cells, null, null);
                }
                break;
            }

            case "image":
                PushFrameImage(node, imageData, builder);
                break;

            case "object":
            case "object-ole":
            {
                string? href = OdfStyles.Attr(node, "href");
                string? formula = href is null ? null
                    : formulaData.GetValueOrDefault(href.StartsWith("./", StringComparison.Ordinal) ? href[2..] : href);
                if (formula is not null) builder.PushFormula(formula, null, null);
                break;
            }
        }
    }

    /// <summary>
    /// Resolve a page-level <c>draw:image</c> against the pre-extracted picture map.
    /// </summary>
    /// <remarks>An unresolvable reference is skipped: the ODT walker already handles images
    /// inside a text box, so only page-level frame images reach here.</remarks>
    private static void PushFrameImage(
        XElement imageNode, Dictionary<string, OdfImage> imageData, InternalDocumentBuilder builder)
    {
        string? href = OdfStyles.Attr(imageNode, "href");
        if (href is null || !imageData.TryGetValue(href, out var img)) return;

        var image = new ExtractedImage
        {
            Data = img.Data,
            Format = img.Format,
            ImageIndex = 0,
            IsMask = false,
        };
        uint idx = builder.PushImage(null, image, null, null);
        builder.SetAttributes(idx, new Dictionary<string, string> { ["src"] = href });
    }

    /// <summary>
    /// Flatten a slide's speaker notes to plain text.
    /// </summary>
    /// <remarks>Notes are kept as a raw block rather than folded into the slide's paragraphs, so
    /// what the audience saw stays distinguishable from what the presenter read.</remarks>
    private static string? ExtractNotesText(XElement notes)
    {
        var paragraphs = new List<string>();
        foreach (var frame in notes.Descendants().Where(n => n.Name.LocalName == "frame"))
        foreach (var textBox in frame.Elements().Where(n => n.Name.LocalName == "text-box"))
        foreach (var p in textBox.Elements().Where(n => n.Name.LocalName is "p" or "h"))
        {
            string? text = OdfContentParser.ExtractNodeText(p)?.Trim();
            if (!string.IsNullOrEmpty(text)) paragraphs.Add(text);
        }
        return paragraphs.Count == 0 ? null : string.Join("\n", paragraphs);
    }

    /// <summary>
    /// Read the static text on each slide master in <c>styles.xml</c> — footers and placeholder
    /// text that every slide using the master displays, and that the content traversal never sees.
    /// </summary>
    private static void ExtractMasterPageText(ZipArchive archive, InternalDocumentBuilder builder)
    {
        string? stylesXml = OdfContentParser.ReadEntry(archive, "styles.xml");
        if (stylesXml is null) return;

        XDocument doc;
        try { doc = Cancel.ParseXml(stylesXml, LoadOptions.PreserveWhitespace); }
        catch (System.Xml.XmlException) { return; }
        if (doc.Root is null) return;

        foreach (var masterPage in doc.Root.Descendants().Where(n => n.Name.LocalName == "master-page"))
        {
            var paragraphs = new List<string>();
            foreach (var p in masterPage.Descendants().Where(n => n.Name.LocalName is "p" or "h"))
            {
                string? text = OdfContentParser.ExtractNodeText(p)?.Trim();
                if (!string.IsNullOrEmpty(text)) paragraphs.Add(text);
            }
            if (paragraphs.Count > 0)
                builder.PushRawBlock("odp-master-page", string.Join("\n", paragraphs), null);
        }
    }
}
