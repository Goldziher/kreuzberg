// Ported from Rust `crates/xberg/src/extractors/odt.rs`.
// Walks content.xml / styles.xml and populates an InternalDocumentBuilder.
// Tracked-changes/revisions are intentionally omitted (InternalDocument.Revisions is
// [JsonIgnore] in the C# port) and image_kind classification is skipped.

using System.IO.Compression;
using System.Text;
using System.Xml.Linq;
using XRay.Content.Internal.MathMarkup;
using XRay.Content.Types;

using XRay.Content.Core;

namespace XRay.Content.Internal.Odf;

/// <summary>Pre-extracted image binary: raw bytes plus a normalized format string.</summary>
internal readonly record struct OdfImage(byte[] Data, string Format);

/// <summary>Native ODF (OpenDocument Text) content parser. Ports the walking logic in odt.rs.</summary>
internal static class OdfContentParser
{
    // ── entry point ────────────────────────────────────────────────────────

    /// <summary>Build an <see cref="InternalDocument"/> from an ODT archive. Ports `build_internal_document`.</summary>
    public static InternalDocument BuildInternalDocument(ZipArchive archive, SecurityLimits? limits = null)
    {
        var budget = new SecurityBudget(limits ?? new SecurityLimits());
        var imageData = PreExtractImages(archive);
        var formulaData = PreExtractFormulas(archive);

        var contentXml = ReadEntry(archive, "content.xml");
        if (contentXml is null)
            return new InternalDocumentBuilder("odt").Build();

        var doc = XDocument.Parse(contentXml, LoadOptions.PreserveWhitespace);
        var root = doc.Root!;
        var styleMap = OdfStyles.BuildStyleMap(root);
        var listStyleMap = OdfStyles.BuildListStyleMap(root);
        // A named list style ("L1") is commonly declared in styles.xml's shared office:styles
        // rather than content.xml's automatic-styles. Merge those in, without letting them
        // override a same-named style content.xml already defined.
        if (ReadEntry(archive, "styles.xml") is { } stylesXmlForLists)
        {
            try
            {
                var stylesDoc = XDocument.Parse(stylesXmlForLists, LoadOptions.PreserveWhitespace);
                if (stylesDoc.Root is not null)
                    foreach (var (name, ordered) in OdfStyles.BuildListStyleMap(stylesDoc.Root))
                        listStyleMap.TryAdd(name, ordered);
            }
            catch (System.Xml.XmlException) { }
        }
        var builder = new InternalDocumentBuilder("odt");

        foreach (var bodyChild in root.Elements())
        {
            if (bodyChild.Name.LocalName != "body")
                continue;
            foreach (var textElem in bodyChild.Elements())
            {
                if (textElem.Name.LocalName != "text")
                    continue;
                BuildInternalElements(textElem, builder, styleMap, listStyleMap, imageData, formulaData, budget);
            }
        }

        ExtractHeadersFooters(archive, builder);
        return builder.Build();
    }

    // ── recursive body walk ────────────────────────────────────────────────

    /// <summary>Ports `build_internal_elements` (revision handling dropped).</summary>
    internal static void BuildInternalElements(
        XElement parent,
        InternalDocumentBuilder builder,
        Dictionary<string, OdtStyleProps> styleMap,
        Dictionary<string, bool> listStyleMap,
        Dictionary<string, OdfImage> imageData,
        Dictionary<string, string> formulaData,
        SecurityBudget budget)
    {
        uint footnoteCounter = 0;

        foreach (var node in parent.Elements())
        {
            budget.Step();
            switch (node.Name.LocalName)
            {
                case "h":
                {
                    var (text, _, uris) = CollectAnnotations(node, styleMap);
                    foreach (var uri in uris)
                        builder.PushUri(uri);
                    var trimmed = text.Trim();
                    if (trimmed.Length > 0)
                    {
                        byte level = 1;
                        var lv = OdfStyles.Attr(node, "outline-level");
                        if (lv is not null && byte.TryParse(lv, out var parsed))
                            level = parsed;
                        builder.PushHeading(level, trimmed, null, null);
                    }
                    break;
                }
                case "p":
                    HandleParagraph(node, builder, styleMap, imageData, formulaData, ref footnoteCounter, budget);
                    break;
                case "table":
                {
                    var cells = ExtractTableCells(node);
                    if (cells.Count > 0)
                    {
                        long cellCount = 0;
                        foreach (var row in cells) cellCount += row.Count;
                        budget.AddCells(cellCount);
                        builder.PushTableFromCells(cells, null, null);
                    }
                    break;
                }
                case "list":
                    BuildList(node, builder, listStyleMap);
                    break;
                case "section":
                    BuildInternalElements(node, builder, styleMap, listStyleMap, imageData, formulaData, budget);
                    break;
            }
        }
    }

    private static void HandleParagraph(
        XElement node,
        InternalDocumentBuilder builder,
        Dictionary<string, OdtStyleProps> styleMap,
        Dictionary<string, OdfImage> imageData,
        Dictionary<string, string> formulaData,
        ref uint footnoteCounter,
        SecurityBudget budget)
    {
        var footnoteMarkers = new List<string>();

        // draw:frame descendants — images or embedded formulas.
        foreach (var desc in DescendantsAndSelf(node))
        {
            if (desc.Name.LocalName != "frame")
                continue;

            bool isFormula = false;
            foreach (var frameChild in desc.Elements())
            {
                if (frameChild.Name.LocalName != "object")
                    continue;
                var objHref = OdfStyles.Attr(frameChild, "href");
                if (objHref is null)
                    continue;
                var normalized = TrimStartStr(objHref, "./");
                if (formulaData.TryGetValue(normalized, out var formulaText))
                {
                    builder.PushFormula(formulaText, null, null);
                    isFormula = true;
                    break;
                }
            }

            if (isFormula)
                continue;

            foreach (var frameChild in DescendantsAndSelf(desc))
            {
                if (frameChild.Name.LocalName != "image")
                    continue;

                // A captioned figure nests a frame inside this one, and the inner frame owns its
                // image. Without this the image is emitted once for the outer frame and again for
                // the inner one.
                bool ownedByNestedFrame = false;
                for (var a = frameChild.Parent; a is not null && a != desc; a = a.Parent)
                    if (a.Name.LocalName == "frame") { ownedByNestedFrame = true; break; }
                if (ownedByNestedFrame)
                    continue;

                var href = OdfStyles.Attr(frameChild, "href");

                // Richer alt text: svg:title/svg:desc/text:p, else frame's svg:title attribute.
                var description = ExtractFrameDescription(desc) ?? OdfStyles.Attr(desc, "title");

                if (href is not null && imageData.TryGetValue(href, out var img))
                {
                    var image = new ExtractedImage
                    {
                        Data = img.Data,
                        Format = img.Format,
                        ImageIndex = 0,
                        IsMask = false,
                        Description = description,
                        // image_kind::classify is intentionally skipped (ImageKind/KindConfidence null).
                    };
                    uint idx = builder.PushImage(description, image, null, null);
                    builder.SetAttributes(idx, new Dictionary<string, string> { ["src"] = href });
                }
                else
                {
                    var textVal = description ?? href ?? "";
                    var elem = InternalElement.TextElement(ElementKind.Image(0), textVal, 0);
                    uint idx = builder.PushElement(elem);
                    if (href is not null)
                        builder.SetAttributes(idx, new Dictionary<string, string> { ["src"] = href });
                }
            }
        }

        // Footnotes: collect citation markers + push definitions on the Footnote layer.
        foreach (var child in DescendantsAndSelf(node))
        {
            if (child.Name.LocalName != "note")
                continue;
            var noteId = OdfStyles.Attr(child, "id");
            // The key prefix carries the note class (#118): "fn" for a footnote, "en" for an
            // endnote. Both the ref and the definition are keyed identically, which is the only
            // thing that tells the two classes apart — no ElementKind distinguishes them.
            string keyPrefix = OdfStyles.Attr(child, "note-class") == "endnote" ? "en" : "fn";

            foreach (var noteChild in child.Elements())
            {
                if (noteChild.Name.LocalName == "note-citation")
                {
                    var citationText = ExtractNodeText(noteChild) ?? "";
                    var citationTrimmed = citationText.Trim();
                    if (citationTrimmed.Length > 0)
                    {
                        footnoteCounter += 1;
                        var key = noteId is not null ? keyPrefix + noteId : $"{keyPrefix}{footnoteCounter}";
                        footnoteMarkers.Add(citationTrimmed);
                        builder.PushFootnoteRef(citationTrimmed, key, null);
                    }
                }
                if (noteChild.Name.LocalName == "note-body")
                {
                    var noteText = ExtractNodeText(noteChild);
                    if (noteText is not null)
                    {
                        var trimmed = noteText.Trim();
                        if (trimmed.Length > 0)
                        {
                            string key;
                            if (noteId is not null)
                            {
                                key = keyPrefix + noteId;
                            }
                            else
                            {
                                if (footnoteCounter == 0)
                                    footnoteCounter += 1;
                                key = $"{keyPrefix}{footnoteCounter}";
                            }
                            uint defIdx = builder.PushFootnoteDefinition(trimmed, key, null);
                            builder.SetLayer(defIdx, ContentLayer.Footnote);
                        }
                    }
                }
            }
        }

        var (paraText, annotations, paraUris) = CollectAnnotations(node, styleMap);
        var text = new StringBuilder(paraText);
        foreach (var uri in paraUris)
            builder.PushUri(uri);

        // Caption paragraphs nested in draw:frame > draw:text-box, missed by CollectAnnotations.
        foreach (var frame in node.Elements())
        {
            if (frame.Name.LocalName != "frame")
                continue;
            foreach (var textBox in frame.Elements())
            {
                if (textBox.Name.LocalName != "text-box")
                    continue;
                foreach (var nestedP in textBox.Elements())
                {
                    if (nestedP.Name.LocalName != "p")
                        continue;
                    var (caption, _, captionUris) = CollectAnnotations(nestedP, styleMap);
                    foreach (var uri in captionUris)
                        builder.PushUri(uri);
                    var captionTrimmed = caption.Trim();
                    if (captionTrimmed.Length > 0)
                    {
                        if (text.Length > 0)
                            text.Append('\n');
                        text.Append(captionTrimmed);
                    }
                }
            }
        }

        // Inject inline footnote markers [^N].
        foreach (var citation in footnoteMarkers)
        {
            var marker = $"[^{citation}]";
            if (text.ToString().IndexOf(marker, StringComparison.Ordinal) < 0)
                text.Append(marker);
        }

        var finalTrimmed = text.ToString().Trim();
        if (finalTrimmed.Length > 0)
        {
            budget.AccountText(System.Text.Encoding.UTF8.GetByteCount(finalTrimmed));
            builder.PushParagraph(finalTrimmed, annotations, null, null);
        }
    }

    /// <summary>
    /// Emit one list, ordered or not according to its own declared style.
    /// </summary>
    /// <remarks>A nested list resolves its own style name rather than inheriting its parent's: a
    /// numbered outline commonly alternates numbered and bulleted levels.</remarks>
    private static void BuildList(
        XElement listNode, InternalDocumentBuilder builder, Dictionary<string, bool> listStyleMap)
    {
        string? styleName = OdfStyles.Attr(listNode, "style-name");
        bool ordered = styleName is not null && listStyleMap.TryGetValue(styleName, out bool o) && o;

        builder.PushList(ordered);
        foreach (var item in listNode.Elements())
        {
            if (item.Name.LocalName != "list-item")
                continue;
            foreach (var child in item.Elements())
            {
                switch (child.Name.LocalName)
                {
                    case "p":
                    case "h":
                    {
                        var text = ExtractNodeText(child);
                        if (text is not null)
                        {
                            var trimmed = text.Trim();
                            if (trimmed.Length > 0)
                                builder.PushListItem(trimmed, ordered, new List<TextAnnotation>(), null, null);
                        }
                        break;
                    }
                    case "list":
                        BuildList(child, builder, listStyleMap);
                        break;
                }
            }
        }
        builder.EndList();
    }

    /// <summary>Ports `extract_odt_internal_headers_footers`. Reads styles.xml header/footer paragraphs.</summary>
    private static void ExtractHeadersFooters(ZipArchive archive, InternalDocumentBuilder builder)
    {
        var stylesXml = ReadEntry(archive, "styles.xml");
        if (stylesXml is null)
            return;

        XDocument doc;
        try
        {
            doc = XDocument.Parse(stylesXml, LoadOptions.PreserveWhitespace);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            return;
        }

        var root = doc.Root;
        if (root is null)
            return;

        foreach (var node in DescendantsAndSelf(root))
        {
            if (node.Name.LocalName == "header")
            {
                var text = ExtractNodeText(node);
                var trimmed = text?.Trim();
                if (!string.IsNullOrEmpty(trimmed))
                {
                    uint idx = builder.PushParagraph(trimmed, new List<TextAnnotation>(), null, null);
                    builder.SetLayer(idx, ContentLayer.Header);
                }
            }
            else if (node.Name.LocalName == "footer")
            {
                var text = ExtractNodeText(node);
                var trimmed = text?.Trim();
                if (!string.IsNullOrEmpty(trimmed))
                {
                    uint idx = builder.PushParagraph(trimmed, new List<TextAnnotation>(), null, null);
                    builder.SetLayer(idx, ContentLayer.Footer);
                }
            }
        }
    }

    // ── text / annotation collection ───────────────────────────────────────

    /// <summary>Ports `collect_odt_annotations`. Byte offsets are UTF-8 byte lengths (Rust `str::len`).</summary>
    private static (string Text, List<TextAnnotation> Annotations, List<ExtractedUri> Uris) CollectAnnotations(
        XElement node,
        Dictionary<string, OdtStyleProps> styleMap)
    {
        var text = new StringBuilder();
        int byteLen = 0;
        var annotations = new List<TextAnnotation>();
        var uris = new List<ExtractedUri>();

        CollectInlineRun(node, styleMap, text, ref byteLen, annotations, uris);

        // Fallback: no children produced text → use the node's own direct text.
        if (text.Length == 0)
        {
            var own = NodeText(node);
            if (own is not null)
                text.Append(own);
        }

        return (text.ToString(), annotations, uris);
    }

    /// <summary>
    /// Ports `collect_inline_run`: walk a paragraph's inline children, flattening their text and
    /// recording byte-offset annotations.
    /// </summary>
    /// <remarks>
    /// Recursion is the point — reading only an element's first text child drops everything past
    /// one level of nesting, so a span inside a span, or a link wrapping a styled span, lost its
    /// tail (upstream #93/#94). It is also why a caption paragraph inside a `draw:text-box` is
    /// picked up here as well as by the caller's dedicated caption pass.
    /// </remarks>
    private static void CollectInlineRun(
        XElement node,
        Dictionary<string, OdtStyleProps> styleMap,
        StringBuilder text,
        ref int byteLen,
        List<TextAnnotation> annotations,
        List<ExtractedUri> uris)
    {
        foreach (var child in node.Nodes())
        {
            if (child is XElement el)
            {
                switch (el.Name.LocalName)
                {
                    case "span":
                    {
                        uint start = (uint)byteLen;
                        CollectInlineRun(el, styleMap, text, ref byteLen, annotations, uris);
                        uint end = (uint)byteLen;
                        if (end == start)
                            continue;

                        var styleName = OdfStyles.Attr(el, "style-name");
                        if (styleName is not null && styleMap.TryGetValue(styleName, out var props))
                        {
                            if (props.Bold)
                                annotations.Add(Ann(start, end, AnnotationKind.Tag.Bold));
                            if (props.Italic)
                                annotations.Add(Ann(start, end, AnnotationKind.Tag.Italic));
                            if (props.Underline)
                                annotations.Add(Ann(start, end, AnnotationKind.Tag.Underline));
                            if (props.Strikethrough)
                                annotations.Add(Ann(start, end, AnnotationKind.Tag.Strikethrough));
                            if (props.Color is not null)
                                annotations.Add(new TextAnnotation
                                {
                                    Start = start,
                                    End = end,
                                    Kind = new AnnotationKind { Which = AnnotationKind.Tag.Color, Value = props.Color },
                                });
                            if (props.FontSize is not null)
                                annotations.Add(new TextAnnotation
                                {
                                    Start = start,
                                    End = end,
                                    Kind = new AnnotationKind { Which = AnnotationKind.Tag.FontSize, Value = props.FontSize },
                                });
                        }
                        break;
                    }
                    case "tab":
                        text.Append('\t');
                        byteLen += 1;
                        break;
                    case "line-break":
                        text.Append('\n');
                        byteLen += 1;
                        break;
                    case "note":
                    case "annotation":
                    case "annotation-end":
                        // Footnotes, endnotes and comments become their own elements.
                        break;
                    case "page-number":
                    case "page-count":
                        // A pagination field caches whatever the authoring application last
                        // displayed there; with no layout pass there is nothing to resolve it to,
                        // and the cached value can be the editor's own placeholder (upstream #69).
                        break;
                    case "a":
                    {
                        int charStart = text.Length;
                        uint start = (uint)byteLen;
                        CollectInlineRun(el, styleMap, text, ref byteLen, annotations, uris);
                        uint end = (uint)byteLen;
                        if (end == start)
                            continue;
                        var linkText = text.ToString(charStart, text.Length - charStart);
                        var url = OdfStyles.Attr(el, "href") ?? "";
                        if (url.Length > 0)
                        {
                            annotations.Add(new TextAnnotation
                            {
                                Start = start,
                                End = end,
                                Kind = new AnnotationKind { Which = AnnotationKind.Tag.Link, Url = url, Title = null },
                            });
                            var kind = url.StartsWith('#') ? UriKind.Anchor
                                : url.StartsWith("mailto:", StringComparison.Ordinal) ? UriKind.Email
                                : UriKind.Hyperlink;
                            uris.Add(new ExtractedUri { Url = url, Label = linkText, Page = null, Kind = kind });
                        }
                        break;
                    }
                    default:
                    {
                        // An unknown wrapper (`text:ruby`, `text:meta`, a `draw:frame` holding a
                        // caption) is descended into rather than dropped.
                        var t = NodeText(el);
                        if (t is not null)
                        {
                            text.Append(t);
                            byteLen += Encoding.UTF8.GetByteCount(t);
                        }
                        else
                        {
                            CollectInlineRun(el, styleMap, text, ref byteLen, annotations, uris);
                        }
                        break;
                    }
                }
            }
            else if (child is XText xt)
            {
                text.Append(xt.Value);
                byteLen += Encoding.UTF8.GetByteCount(xt.Value);
            }
        }
    }

    /// <summary>Ports `extract_node_text`: concatenates span/tab/line-break/direct-text of the node's children.</summary>
    internal static string? ExtractNodeText(XElement node)
    {
        var parts = new List<string>();
        foreach (var child in node.Nodes())
        {
            if (child is XElement el)
            {
                switch (el.Name.LocalName)
                {
                    case "span":
                    {
                        var t = NodeText(el);
                        if (t is not null)
                            parts.Add(t);
                        break;
                    }
                    case "tab":
                        parts.Add("\t");
                        break;
                    case "line-break":
                        parts.Add("\n");
                        break;
                    // A comment's body is not part of the text it annotates.
                    case "annotation":
                    case "annotation-end":
                        break;
                    // A field carries its last-rendered value as text — a slide master's page
                    // number placeholder reads "<number>". That is the application's cache of
                    // what the field showed, not something anyone wrote, and emitting it puts a
                    // literal "<number>" in the extracted text of every presentation.
                    case "page-number":
                    case "page-count":
                        break;
                    default:
                    {
                        var t = NodeText(el);
                        if (t is not null)
                            parts.Add(t);
                        break;
                    }
                }
            }
            else if (child is XText xt)
            {
                parts.Add(xt.Value);
            }
        }

        if (parts.Count == 0)
            return NodeText(node);
        return string.Concat(parts);
    }

    /// <summary>Ports `extract_frame_description`: svg:title, then svg:desc, then text:p children.</summary>
    private static string? ExtractFrameDescription(XElement frame)
    {
        foreach (var child in frame.Elements())
        {
            if (child.Name.LocalName == "title")
            {
                var t = NodeText(child)?.Trim();
                if (!string.IsNullOrEmpty(t))
                    return t;
            }
        }
        foreach (var child in frame.Elements())
        {
            if (child.Name.LocalName == "desc")
            {
                var t = NodeText(child)?.Trim();
                if (!string.IsNullOrEmpty(t))
                    return t;
            }
        }
        foreach (var child in frame.Elements())
        {
            if (child.Name.LocalName == "p")
            {
                var t = ExtractNodeText(child)?.Trim();
                if (!string.IsNullOrEmpty(t))
                    return t;
            }
        }
        return null;
    }

    // ── tables ─────────────────────────────────────────────────────────────

    /// <summary>Ports `extract_table_cells`. Handles direct rows and table-header-rows containers.</summary>
    internal static List<List<string>> ExtractTableCells(XElement tableNode)
    {
        var rows = new List<List<string>>();
        foreach (var child in tableNode.Elements())
        {
            switch (child.Name.LocalName)
            {
                case "table-row":
                {
                    var row = ExtractRowCells(child);
                    if (row is not null)
                        rows.Add(row);
                    break;
                }
                case "table-header-rows":
                {
                    foreach (var rowNode in child.Elements())
                    {
                        if (rowNode.Name.LocalName != "table-row")
                            continue;
                        var row = ExtractRowCells(rowNode);
                        if (row is not null)
                            rows.Add(row);
                    }
                    break;
                }
            }
        }
        return rows;
    }

    private static List<string>? ExtractRowCells(XElement rowNode)
    {
        var rowCells = new List<string>();
        foreach (var cellNode in rowNode.Elements())
        {
            if (cellNode.Name.LocalName != "table-cell")
                continue;
            var cellText = ExtractNodeText(cellNode) ?? "";
            rowCells.Add(cellText.Trim());
        }
        return rowCells.Count == 0 ? null : rowCells;
    }

    // ── image / formula pre-extraction ─────────────────────────────────────

    /// <summary>Ports `pre_extract_images`. Maps <c>Pictures/*</c> href → (bytes, format).</summary>
    internal static Dictionary<string, OdfImage> PreExtractImages(ZipArchive archive)
    {
        var images = new Dictionary<string, OdfImage>();
        foreach (var entry in archive.Entries)
        {
            var name = entry.FullName;
            if (!name.StartsWith("Pictures/", StringComparison.Ordinal))
                continue;

            int dot = name.LastIndexOf('.');
            var ext = dot >= 0 ? name[(dot + 1)..].ToLowerInvariant() : "";
            var format = ext switch
            {
                "jpg" or "jpeg" => "jpeg",
                "png" => "png",
                "gif" => "gif",
                "webp" => "webp",
                "svg" => "svg",
                "bmp" => "bmp",
                "tiff" or "tif" => "tiff",
                _ => "png",
            };

            var buf = ReadEntryBytes(entry);
            if (buf.Length > 0)
                images[name] = new OdfImage(buf, format);
        }
        return images;
    }

    /// <summary>Ports `pre_extract_formulas`. Maps embedded object dirs → the formula's LaTeX.</summary>
    internal static Dictionary<string, string> PreExtractFormulas(ZipArchive archive)
    {
        var formulas = new Dictionary<string, string>();
        foreach (var entry in archive.Entries)
        {
            var name = entry.FullName;
            if (!name.EndsWith("/content.xml", StringComparison.Ordinal) || name == "content.xml")
                continue;

            string xml;
            using (var stream = entry.Open())
            using (var reader = new StreamReader(stream))
                xml = reader.ReadToEnd();

            if (!xml.Contains("math", StringComparison.Ordinal))
                continue;

            var text = MathMl.ConvertMathmlStrToLatex(xml);
            if (text.Length == 0)
                continue;

            var dir = name[..^"/content.xml".Length];
            formulas[dir] = text;
            formulas[dir + "/"] = text;
        }
        return formulas;
    }

    // ── small helpers ──────────────────────────────────────────────────────

    /// <summary>
    /// Mirrors roxmltree <c>Node::text()</c> for an element: the value of the first child node
    /// when that child is a text (or CDATA) node, otherwise null.
    /// </summary>
    internal static string? NodeText(XElement el)
    {
        var first = el.FirstNode;
        return first is XText t ? t.Value : null;
    }

    private static IEnumerable<XElement> DescendantsAndSelf(XElement el)
    {
        yield return el;
        foreach (var d in el.Descendants())
            yield return d;
    }

    private static TextAnnotation Ann(uint start, uint end, AnnotationKind.Tag tag) =>
        new() { Start = start, End = end, Kind = new AnnotationKind { Which = tag } };

    // Mirrors Rust `str::trim_start_matches`: strips every leading occurrence of the prefix.
    private static string TrimStartStr(string s, string prefix)
    {
        while (prefix.Length > 0 && s.StartsWith(prefix, StringComparison.Ordinal))
            s = s[prefix.Length..];
        return s;
    }

    internal static string? ReadEntry(ZipArchive archive, string name)
    {
        var entry = archive.GetEntry(name);
        if (entry is null)
            return null;
        using var stream = entry.Open();
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }

    private static byte[] ReadEntryBytes(ZipArchiveEntry entry)
    {
        using var stream = entry.Open();
        using var ms = new MemoryStream();
        stream.CopyTo(ms);
        return ms.ToArray();
    }
}
