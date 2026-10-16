using System.Text;
using XRay.Content.Internal.DocTags;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>
/// Render an <see cref="InternalDocument"/> to Docling DocTags, ported from Rust
/// <c>rendering/doctags.rs</c>. The inverse of <c>DocTagsParser</c>.
/// </summary>
public static class DocTagsRenderer
{
    /// <summary>Language token for a code block with no recorded language.</summary>
    private const string UnknownLanguage = "<_unknown_>";

    private const double LocGrid = DocTagsVocabulary.LocGrid;

    public static string Render(InternalDocument doc)
    {
        var output = new StringBuilder();
        var captions = CollectCaptions(doc);
        var dims = PageDimensions(doc);

        output.Append("<doctag>");

        var state = new ListState();

        for (int i = 0; i < doc.Elements.Count; i++)
        {
            Cancel.Every(i, 0x3FF);
            uint index = (uint)i;
            var elem = doc.Elements[i];

            // A caption is emitted nested inside the element it describes.
            if (captions.Sources.ContainsKey(index)) continue;

            string? loc = ElementLoc(elem, dims);

            switch (elem.Kind.Tag)
            {
                case ElementKindTag.ListItem:
                {
                    state.Open(output, elem.Kind.Ordered);
                    // DocTags' list_item tag carries no separate marker slot — a literal source
                    // label the auto `ordered` container alone cannot express is prefixed onto the
                    // visible text, exactly as the other text-based renderers do.
                    string itemText = elem.ListItemSourceLabel is { Length: > 0 } tagLabel
                        ? tagLabel + " " + elem.Text
                        : elem.Text;
                    PushElement(output, "list_item", loc,
                                RenderCommon.NormalizeInlineText(itemText), null);
                    continue;
                }
                case ElementKindTag.ListStart:
                    state.OpenExplicit(output, elem.Kind.Ordered);
                    continue;
                case ElementKindTag.ListEnd:
                    state.CloseExplicit(output);
                    continue;
                default:
                    state.CloseImplicit(output);
                    break;
            }

            switch (elem.Kind.Tag)
            {
                case ElementKindTag.QuoteStart:
                case ElementKindTag.QuoteEnd:
                case ElementKindTag.GroupStart:
                case ElementKindTag.GroupEnd:
                    break;

                // The marker itself carries no content DocTags can address — the reference is
                // resolved through a Relationship, and the definition is emitted on its own
                // below. CommentDefinition is deliberately NOT dropped alongside these: the
                // other renderers re-emit comment bodies in a later pass, and this one has no
                // second pass, so dropping it here would make DocTags the only renderer that
                // loses the text outright.
                case ElementKindTag.FootnoteRef:
                case ElementKindTag.CommentRef:
                    break;

                case ElementKindTag.PageBreak:
                    output.Append("<page_break>\n");
                    break;

                case ElementKindTag.Table:
                {
                    bool rendered = false;
                    int tableIndex = (int)elem.Kind.TableIndex;
                    if (tableIndex >= 0 && tableIndex < doc.Tables.Count)
                    {
                        string? caption = captions.CaptionPayload(doc, index, dims);
                        rendered = PushOtsl(output, doc.Tables[tableIndex].Cells, loc, caption);
                    }
                    // A dropped table would take its caption down with it — unlike Image and
                    // Code, which always call PushElement and so always carry their nested
                    // caption. The parser has the same rule in the other direction.
                    if (!rendered) PushOrphanedCaption(output, doc, captions, index, dims);
                    break;
                }

                case ElementKindTag.Image:
                {
                    int imageIndex = (int)elem.Kind.ImageIndex;
                    string? described = imageIndex >= 0 && imageIndex < doc.Images.Count
                        ? doc.Images[imageIndex].Description
                        : null;
                    if (described is { Length: 0 }) described = null;
                    string? caption = captions.CaptionPayload(doc, index, dims)
                                      ?? (described is null ? null : RenderCommon.NormalizeInlineText(described));
                    PushElement(output, "picture", loc, "", caption);
                    break;
                }

                case ElementKindTag.Code:
                {
                    string body = CodeBody(RenderCommon.GetLanguage(elem), elem.Text);
                    PushElement(output, "code", loc, body, captions.CaptionPayload(doc, index, dims));
                    break;
                }

                case ElementKindTag.Formula:
                    PushElement(output, "formula", loc, RenderCommon.NormalizeInlineText(elem.Text), null);
                    break;

                case ElementKindTag.Admonition:
                {
                    // The builder stores exactly one string: `Text` is the title or, failing
                    // that, the kind. There is no separate body to distinguish from the label,
                    // and DocTags has no admonition tag, so this is an ordinary text element,
                    // emitted exactly once.
                    string label = RenderCommon.GetAdmonitionTitle(elem)
                                   ?? RenderCommon.GetAdmonitionKind(elem);
                    PushTextElement(output, elem.Layer, loc, RenderCommon.NormalizeInlineText(label));
                    break;
                }

                case ElementKindTag.MetadataBlock:
                {
                    var entries = RenderCommon.ParseMetadataEntries(elem.Text);
                    if (entries.Count == 0)
                        PushTextElement(output, elem.Layer, loc, RenderCommon.NormalizeInlineText(elem.Text));
                    else
                        foreach (var (key, value) in entries)
                            PushTextElement(output, elem.Layer, loc, $"{key}: {value}");
                    break;
                }

                case ElementKindTag.RawBlock:
                    PushElement(output, "code", loc, CodeBody(null, elem.Text), null);
                    break;

                case ElementKindTag.Title:
                    PushLabelled(output, elem.Layer, loc, "title", RenderCommon.NormalizeInlineText(elem.Text));
                    break;

                case ElementKindTag.Heading:
                    PushLabelled(output, elem.Layer, loc,
                                 $"section_header_level_{Math.Max((byte)1, elem.Kind.Level)}",
                                 RenderCommon.NormalizeInlineText(elem.Text));
                    break;

                case ElementKindTag.FootnoteDefinition:
                    PushElement(output, "footnote", loc, RenderCommon.NormalizeInlineText(elem.Text), null);
                    break;

                // DocTags has no comment tag, so a comment definition renders through its
                // content layer: `<footnote>` when the extractor marked it as such, `<text>`
                // otherwise. See the note above on why it is not dropped.
                case ElementKindTag.CommentDefinition:
                case ElementKindTag.Paragraph:
                case ElementKindTag.Citation:
                case ElementKindTag.Slide:
                case ElementKindTag.DefinitionTerm:
                case ElementKindTag.DefinitionDescription:
                case ElementKindTag.OcrText:
                    PushTextElement(output, elem.Layer, loc, RenderCommon.NormalizeInlineText(elem.Text));
                    break;
            }
        }

        state.CloseImplicit(output);
        state.CloseAll(output);

        output.Append("</doctag>");
        return output.ToString();
    }

    /// <summary>
    /// Tracks open <c>&lt;ordered_list&gt;</c> / <c>&lt;unordered_list&gt;</c> wrappers.
    /// </summary>
    /// <remarks>
    /// Extractors emit list items either wrapped in explicit ListStart/ListEnd markers or bare.
    /// A bare item opens an implicit wrapper that closes at the next non-list element.
    /// </remarks>
    private sealed class ListState
    {
        private readonly List<bool> _explicit = new();
        private bool? _implicit;

        public void Open(StringBuilder output, bool ordered)
        {
            if (_explicit.Count > 0) return;
            // A bare item whose ordering differs from the open implicit wrapper must close that
            // wrapper and open the right kind, rather than being absorbed into the wrong one.
            if (_implicit == ordered) return;
            CloseImplicit(output);
            PushListOpen(output, ordered);
            _implicit = ordered;
        }

        public void OpenExplicit(StringBuilder output, bool ordered)
        {
            CloseImplicit(output);
            PushListOpen(output, ordered);
            _explicit.Add(ordered);
        }

        public void CloseExplicit(StringBuilder output)
        {
            if (_explicit.Count == 0) return;
            bool ordered = _explicit[^1];
            _explicit.RemoveAt(_explicit.Count - 1);
            PushListClose(output, ordered);
        }

        public void CloseImplicit(StringBuilder output)
        {
            if (_implicit is not { } ordered) return;
            _implicit = null;
            PushListClose(output, ordered);
        }

        public void CloseAll(StringBuilder output)
        {
            while (_explicit.Count > 0) CloseExplicit(output);
        }
    }

    private static void PushListOpen(StringBuilder output, bool ordered) =>
        output.Append(ordered ? "<ordered_list>" : "<unordered_list>");

    private static void PushListClose(StringBuilder output, bool ordered) =>
        output.Append(ordered ? "</ordered_list>\n" : "</unordered_list>\n");

    /// <summary>
    /// Per-page dimensions in points, keyed by 1-indexed page number.
    /// </summary>
    /// <remarks>
    /// Only pages that recorded usable dimensions are included, and the emptiness of this map is
    /// what suppresses location tokens for formats that have none.
    /// </remarks>
    private static Dictionary<uint, (double Width, double Height)> PageDimensions(InternalDocument doc)
    {
        var dims = new Dictionary<uint, (double, double)>();
        if (doc.Metadata.Pages?.Pages is not { } pages) return dims;

        foreach (var entry in pages)
        {
            if (entry is not PageInfoDto info || info.Dimensions is not { } d) continue;
            if (double.IsFinite(d.Width) && double.IsFinite(d.Height) && d.Width > 0 && d.Height > 0)
                dims[info.Number] = d;
        }
        return dims;
    }

    /// <summary>
    /// An element's <c>&lt;loc_*&gt;</c> tokens, or null when the geometry is unusable.
    /// </summary>
    /// <remarks>
    /// Input boxes follow the <see cref="BoundingBox"/> contract — PDF user space, origin at the
    /// bottom-left — and DocTags counts from the top-left, so the vertical axis is flipped here.
    /// Tokens are emitted left, top, right, bottom.
    /// <para>
    /// A format whose boxes do not follow that contract must not reach here. PPTX is the live
    /// example: it fills <c>Y0</c> with the top edge, so its geometry would come out flipped. It
    /// is excluded structurally rather than by a format check — PPTX records no page dimensions,
    /// so the map above yields nothing for it and no tokens are produced.
    /// </para>
    /// </remarks>
    private static string? LocTokens(BoundingBox bbox, (double Width, double Height) page)
    {
        if (!double.IsFinite(bbox.X0) || !double.IsFinite(bbox.Y0)
            || !double.IsFinite(bbox.X1) || !double.IsFinite(bbox.Y1)) return null;

        // Tolerate boxes recorded with reversed corners.
        double left = Math.Min(bbox.X0, bbox.X1);
        double right = Math.Max(bbox.X0, bbox.X1);
        double bottom = Math.Min(bbox.Y0, bbox.Y1);
        double top = Math.Max(bbox.Y0, bbox.Y1);

        static uint ToGrid(double value, double extent) =>
            (uint)Math.Clamp(Math.Round(value / extent * LocGrid, MidpointRounding.AwayFromZero), 0.0, LocGrid);

        return $"<loc_{ToGrid(left, page.Width)}><loc_{ToGrid(page.Height - top, page.Height)}>"
               + $"<loc_{ToGrid(right, page.Width)}><loc_{ToGrid(page.Height - bottom, page.Height)}>";
    }

    private static string? ElementLoc(InternalElement elem, Dictionary<uint, (double, double)> dims)
    {
        if (elem.Bbox is not { } bbox || elem.Page is not { } page) return null;
        return dims.TryGetValue(page, out var d) ? LocTokens(bbox, d) : null;
    }

    /// <summary>A code body: the language token, then the flattened source.</summary>
    private static string CodeBody(string? language, string text) =>
        (language is null ? UnknownLanguage : $"<_{language}_>") + RenderCommon.NormalizeInlineText(text);

    /// <summary>Emit an element: its tag, location tokens, body, and any nested caption.</summary>
    private static void PushElement(
        StringBuilder output, string tag, string? loc, string body, string? caption)
    {
        output.Append('<').Append(tag).Append('>');
        if (loc is not null) output.Append(loc);
        output.Append(body);
        if (caption is not null) output.Append("<caption>").Append(caption).Append("</caption>");
        output.Append("</").Append(tag).Append(">\n");
    }

    /// <summary>Emit a prose element, letting the content layer choose the tag.</summary>
    private static void PushTextElement(StringBuilder output, ContentLayer layer, string? loc, string text)
    {
        if (text.Length == 0) return;
        PushElement(output, LayerTag(layer, "text"), loc, text, null);
    }

    /// <summary>Emit an element whose tag a non-body content layer overrides.</summary>
    private static void PushLabelled(
        StringBuilder output, ContentLayer layer, string? loc, string tag, string text)
    {
        if (text.Length == 0) return;
        PushElement(output, LayerTag(layer, tag), loc, text, null);
    }

    private static string LayerTag(ContentLayer layer, string bodyTag) => layer switch
    {
        ContentLayer.Header => "page_header",
        ContentLayer.Footer => "page_footer",
        ContentLayer.Footnote => "footnote",
        _ => bodyTag,
    };

    /// <summary>
    /// Serialise a flat cell grid as OTSL, with the first row as the header.
    /// </summary>
    /// <remarks>
    /// Ragged rows are padded with <c>&lt;ecel&gt;</c>, which OTSL requires. Returns false
    /// without writing anything for a table that is empty or has no columns — the caller has to
    /// handle that, or a caption that would have nested inside the dropped table goes with it.
    /// </remarks>
    private static bool PushOtsl(
        StringBuilder output, List<List<string>> cells, string? loc, string? caption)
    {
        if (cells.Count == 0) return false;
        int columns = 0;
        foreach (var row in cells) columns = Math.Max(columns, row.Count);
        if (columns == 0) return false;

        output.Append("<otsl>");
        if (loc is not null) output.Append(loc);

        for (int r = 0; r < cells.Count; r++)
        {
            for (int c = 0; c < columns; c++)
            {
                string content = c < cells[r].Count ? cells[r][c].Trim() : "";
                if (content.Length == 0) output.Append("<ecel>");
                else output.Append(r == 0 ? "<ched>" : "<fcel>")
                           .Append(RenderCommon.NormalizeInlineText(content));
            }
            output.Append("<nl>");
        }

        if (caption is not null) output.Append("<caption>").Append(caption).Append("</caption>");
        output.Append("</otsl>\n");
        return true;
    }

    /// <summary>Caption relationships, indexed both ways.</summary>
    private sealed class Captions
    {
        /// <summary>Caption element index to the element it describes.</summary>
        public readonly Dictionary<uint, uint> Sources = new();

        /// <summary>Described element index to its caption element index.</summary>
        public readonly Dictionary<uint, uint> Targets = new();

        /// <summary>The inner content of a caption: its own location tokens, then its text.</summary>
        public string? CaptionPayload(
            InternalDocument doc, uint target, Dictionary<uint, (double, double)> dims)
        {
            if (!Targets.TryGetValue(target, out uint source)) return null;
            if (source >= doc.Elements.Count) return null;
            var elem = doc.Elements[(int)source];
            string text = RenderCommon.NormalizeInlineText(elem.Text);
            if (text.Length == 0) return null;
            string? loc = ElementLoc(elem, dims);
            return loc is null ? text : loc + text;
        }
    }

    /// <summary>
    /// Render a caption as an ordinary text element when its target did not render — an empty
    /// table dropped by <c>PushOtsl</c>, say. The parser has the mirror rule: a caption whose
    /// target was dropped still carries text, so it stays rather than going down with the target.
    /// </summary>
    private static void PushOrphanedCaption(
        StringBuilder output, InternalDocument doc, Captions captions, uint target,
        Dictionary<uint, (double, double)> dims)
    {
        if (!captions.Targets.TryGetValue(target, out uint source)) return;
        if (source >= doc.Elements.Count) return;
        var elem = doc.Elements[(int)source];
        PushTextElement(output, elem.Layer, ElementLoc(elem, dims),
                        RenderCommon.NormalizeInlineText(elem.Text));
    }

    private static Captions CollectCaptions(InternalDocument doc)
    {
        var captions = new Captions();

        foreach (var rel in doc.Relationships)
        {
            if (rel.Kind != RelationshipKind.Caption) continue;
            if (rel.Target.Index is not { } target) continue;

            // Only tags that nest a caption may claim one. Captions also attach to plain
            // paragraphs standing in for a figure (a LaTeX `figure` with placeholder injection),
            // and those have to keep rendering as their own element rather than being swallowed.
            bool nestsCaption = target < doc.Elements.Count
                                && doc.Elements[(int)target].Kind.Tag
                                   is ElementKindTag.Table or ElementKindTag.Image or ElementKindTag.Code;
            if (!nestsCaption) continue;

            // First caption wins, so a described element never gains two.
            if (captions.Targets.ContainsKey(target)) continue;

            captions.Sources[rel.Source] = target;
            captions.Targets[target] = rel.Source;
        }

        return captions;
    }
}
