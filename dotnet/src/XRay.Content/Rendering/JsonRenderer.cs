using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>
/// JSON tree renderer for <see cref="InternalDocument"/>. Produces a heading-driven section
/// tree serialized as <c>{"type":"section"|"paragraph"|...}</c>. Ported verbatim from `rendering/json.rs`.
/// Uses relaxed escaping so the emitted string matches serde_json (which does not escape &lt; &gt; &amp; /).
/// </summary>
public static class JsonRenderer
{
    private static readonly JsonSerializerOptions WriteOptions = new()
    {
        Encoder = SerdeJsonEncoder.Shared,
        WriteIndented = false,
    };

    private sealed class OpenSection
    {
        public string Heading = "";
        public byte Level;
        public JsonArray Body = new();
    }

    private sealed class OpenList
    {
        public bool Ordered;
        public JsonArray Items = new();
    }

    public static string Render(InternalDocument doc)
    {
        var jsonDoc = BuildJsonDocument(doc);
        return jsonDoc.ToJsonString(WriteOptions);
    }

    private static JsonObject BuildJsonDocument(InternalDocument doc)
    {
        string? title = null;
        var sectionStack = new List<OpenSection>();
        var rootBody = new JsonArray();
        var state = new RenderState();
        OpenList? openList = null;
        JsonArray? openBlockquote = null;
        var footnotes = new FootnoteCollector(doc);

        for (int elemIndex = 0; elemIndex < doc.Elements.Count; elemIndex++)
        {
            Cancel.Every(elemIndex, 0x3FF);
            var elem = doc.Elements[elemIndex];
            if (!RenderCommon.IsBodyElement(elem)) continue;

            if (RenderCommon.IsContainerEnd(elem))
            {
                if (elem.Kind.Tag == ElementKindTag.ListEnd)
                {
                    if (openList is not null)
                    {
                        PushToCurrent(rootBody, sectionStack, ref openBlockquote, ListNode(openList));
                        openList = null;
                    }
                }
                else if (elem.Kind.Tag == ElementKindTag.QuoteEnd)
                {
                    if (openBlockquote is not null)
                    {
                        var bq = openBlockquote;
                        openBlockquote = null;
                        JsonArray? none = null;
                        PushToCurrent(rootBody, sectionStack, ref none, BlockquoteNode(bq));
                    }
                }
                RenderCommon.HandleContainerEnd(elem.Kind, state);
                continue;
            }

            switch (elem.Kind.Tag)
            {
                case ElementKindTag.Title:
                    if (title is null && elem.Text.Length > 0) title = elem.Text;
                    break;

                case ElementKindTag.Heading:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    CloseSectionsToLevel(sectionStack, rootBody, elem.Kind.Level);
                    sectionStack.Add(new OpenSection { Heading = elem.Text, Level = elem.Kind.Level, Body = new() });
                    break;

                case ElementKindTag.Paragraph:
                    if (elem.Text.Length == 0) continue;
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote, ParagraphNode(elem.Text));
                    break;

                case ElementKindTag.ListStart:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    state.PushContainer(NestingKind.ListKind(elem.Kind.Ordered, 0), elem.Depth);
                    openList = new OpenList { Ordered = elem.Kind.Ordered };
                    break;

                case ElementKindTag.ListItem:
                {
                    // JSON list items are plain strings with no separate marker slot, so a literal
                    // source label is prefixed onto the item text — the same convention the other
                    // text-based renderers use.
                    string itemText = elem.ListItemSourceLabel is { Length: > 0 } jsonLabel
                        ? jsonLabel + " " + elem.Text
                        : elem.Text;
                    if (openList is not null)
                        openList.Items.Add(itemText);
                    else
                    {
                        var single = new OpenList { Ordered = elem.Kind.Ordered };
                        single.Items.Add(itemText);
                        PushToCurrent(rootBody, sectionStack, ref openBlockquote, ListNode(single));
                    }
                    break;
                }

                case ElementKindTag.Code:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote, CodeNode(elem.Text, RenderCommon.GetLanguage(elem)));
                    break;

                case ElementKindTag.Formula:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote, FormulaNode(elem.Text));
                    break;

                case ElementKindTag.Table:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    {
                        int ti = (int)elem.Kind.TableIndex;
                        if (ti >= 0 && ti < doc.Tables.Count)
                            PushToCurrent(rootBody, sectionStack, ref openBlockquote, TableNode(doc.Tables[ti]));
                    }
                    break;

                case ElementKindTag.Image:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    {
                        int ii = (int)elem.Kind.ImageIndex;
                        ExtractedImage? image = ii >= 0 && ii < doc.Images.Count ? doc.Images[ii] : null;
                        string? alt = image?.Description;
                        string? src = null;
                        if (image is not null)
                        {
                            src = image.Data.Length > 0
                                ? $"image_{elem.Kind.ImageIndex}.{image.Format}"
                                : image.SourcePath;
                        }
                        PushToCurrent(rootBody, sectionStack, ref openBlockquote, ImageNode(alt, src));
                    }
                    break;

                case ElementKindTag.QuoteStart:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    state.PushContainer(NestingKind.BlockQuote, elem.Depth);
                    openBlockquote = new JsonArray();
                    break;

                case ElementKindTag.OcrText:
                    if (elem.Text.Length > 0)
                        PushToCurrent(rootBody, sectionStack, ref openBlockquote, ParagraphNode(elem.Text));
                    break;

                case ElementKindTag.PageBreak:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote, PageBreakNode(elem.Page));
                    break;

                case ElementKindTag.FootnoteRef:
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        FootnoteRefNode(footnotes.RefNumber((uint)elemIndex), elem.Anchor));
                    break;

                case ElementKindTag.FootnoteDefinition:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        FootnoteDefinitionNode(elem.Text, elem.Anchor));
                    break;

                case ElementKindTag.CommentRef:
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote, CommentRefNode(elem.Anchor));
                    break;

                case ElementKindTag.CommentDefinition:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        CommentDefinitionNode(elem.Text, elem.Anchor));
                    break;

                case ElementKindTag.Citation:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        CitationNode(elem.Text, elem.Anchor));
                    break;

                case ElementKindTag.Slide:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        SlideNode(elem.Kind.Number, elem.Text.Length == 0 ? null : elem.Text));
                    break;

                case ElementKindTag.DefinitionTerm:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        TextNode("definition_term", elem.Text));
                    break;

                case ElementKindTag.DefinitionDescription:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        TextNode("definition_description", elem.Text));
                    break;

                case ElementKindTag.Admonition:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote, AdmonitionNode(
                        RenderCommon.GetAdmonitionKind(elem), RenderCommon.GetAdmonitionTitle(elem), elem.Text));
                    break;

                case ElementKindTag.RawBlock:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        RawBlockNode(elem.Text, elem.Attributes is not null && elem.Attributes.TryGetValue("format", out var rawFormat) ? rawFormat : null));
                    break;

                case ElementKindTag.MetadataBlock:
                    FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);
                    PushToCurrent(rootBody, sectionStack, ref openBlockquote,
                        MetadataBlockNode(RenderCommon.ParseMetadataEntries(elem.Text), elem.Text));
                    break;

                // ListEnd / QuoteEnd / GroupStart / GroupEnd carry no node of their own.
            }
        }

        FlushList(ref openList, rootBody, sectionStack, ref openBlockquote);

        if (openBlockquote is not null)
        {
            var bq = openBlockquote;
            openBlockquote = null;
            JsonArray? none = null;
            PushToCurrent(rootBody, sectionStack, ref none, BlockquoteNode(bq));
        }

        CloseSectionsToLevel(sectionStack, rootBody, 0);

        // Footnote definitions live on the Footnote layer, not Body, so the loop above never
        // sees them — `IsBodyElement` filters them out. That filter is correct (a definition is
        // document furniture, not body flow, and must not be interleaved into whatever section
        // happened to be open), but it also dropped the definition a `[^n]` marker points at.
        // Append them once, in document order, at the end — mirroring the Markdown renderer.
        foreach (var elem in doc.Elements)
        {
            if (elem.Kind.Tag == ElementKindTag.FootnoteDefinition && !RenderCommon.IsBodyElement(elem))
                rootBody.Add(FootnoteDefinitionNode(elem.Text, elem.Anchor));
            if (elem.Kind.Tag == ElementKindTag.CommentDefinition && !RenderCommon.IsBodyElement(elem))
                rootBody.Add(CommentDefinitionNode(elem.Text, elem.Anchor));
        }

        var result = new JsonObject();
        if (title is not null) result["title"] = title;
        result["body"] = rootBody;
        if (doc.Annotations is { Count: > 0 } annotations)
            result["annotations"] = JsonSerializer.SerializeToNode(annotations, XRay.Content.Types.Json.Options);
        return result;
    }

    private static void PushToCurrent(JsonArray rootBody, List<OpenSection> sectionStack,
        ref JsonArray? openBlockquote, JsonNode node)
    {
        if (openBlockquote is not null)
        {
            openBlockquote.Add(node);
            return;
        }
        if (sectionStack.Count > 0)
            sectionStack[^1].Body.Add(node);
        else
            rootBody.Add(node);
    }

    private static void CloseSectionsToLevel(List<OpenSection> sectionStack, JsonArray rootBody, byte targetLevel)
    {
        while (sectionStack.Count > 0)
        {
            var top = sectionStack[^1];
            if (top.Level >= targetLevel)
            {
                sectionStack.RemoveAt(sectionStack.Count - 1);
                var node = new JsonObject
                {
                    ["type"] = "section",
                    ["heading"] = top.Heading,
                    ["level"] = top.Level,
                    ["body"] = top.Body,
                };
                if (sectionStack.Count > 0)
                    sectionStack[^1].Body.Add(node);
                else
                    rootBody.Add(node);
            }
            else break;
        }
    }

    private static void FlushList(ref OpenList? openList, JsonArray rootBody,
        List<OpenSection> sectionStack, ref JsonArray? openBlockquote)
    {
        if (openList is not null)
        {
            var node = ListNode(openList);
            openList = null;
            PushToCurrent(rootBody, sectionStack, ref openBlockquote, node);
        }
    }

    private static JsonObject TextNode(string type, string text) => new() { ["type"] = type, ["text"] = text };

    private static JsonObject PageBreakNode(uint? page)
    {
        var o = new JsonObject { ["type"] = "page_break" };
        if (page is { } p) o["page"] = p;
        return o;
    }

    private static JsonObject FootnoteRefNode(uint? number, string? id)
    {
        var o = new JsonObject { ["type"] = "footnote_ref" };
        if (number is { } n) o["number"] = n;
        if (id is not null) o["id"] = id;
        return o;
    }

    private static JsonObject CommentRefNode(string? id)
    {
        var node = new JsonObject { ["type"] = "comment_ref" };
        if (id is not null) node["id"] = id;
        return node;
    }

    private static JsonObject CommentDefinitionNode(string text, string? id)
    {
        var node = new JsonObject { ["type"] = "comment_definition", ["text"] = text };
        if (id is not null) node["id"] = id;
        return node;
    }

    private static JsonObject FootnoteDefinitionNode(string text, string? id)
    {
        var o = new JsonObject { ["type"] = "footnote_definition", ["text"] = text };
        if (id is not null) o["id"] = id;
        return o;
    }

    private static JsonObject CitationNode(string text, string? id)
    {
        var o = new JsonObject { ["type"] = "citation", ["text"] = text };
        if (id is not null) o["id"] = id;
        return o;
    }

    private static JsonObject SlideNode(uint number, string? title)
    {
        var o = new JsonObject { ["type"] = "slide", ["number"] = number };
        if (title is not null) o["title"] = title;
        return o;
    }

    private static JsonObject AdmonitionNode(string kind, string? title, string text)
    {
        var o = new JsonObject { ["type"] = "admonition", ["kind"] = kind };
        if (title is not null) o["title"] = title;
        o["text"] = text;
        return o;
    }

    private static JsonObject RawBlockNode(string text, string? format)
    {
        var o = new JsonObject { ["type"] = "raw_block", ["text"] = text };
        if (format is not null) o["format"] = format;
        return o;
    }

    /// <summary>Raw block text is emitted only when no `key: value` entries could be parsed.</summary>
    private static JsonObject MetadataBlockNode(List<(string Key, string Value)> entries, string rawText)
    {
        var arr = new JsonArray();
        foreach (var (key, value) in entries)
            arr.Add(new JsonObject { ["key"] = key, ["value"] = value });
        var o = new JsonObject { ["type"] = "metadata_block", ["entries"] = arr };
        if (entries.Count == 0 && rawText.Length > 0) o["text"] = rawText;
        return o;
    }

    private static JsonObject ParagraphNode(string text) => new() { ["type"] = "paragraph", ["text"] = text };

    private static JsonObject FormulaNode(string text) => new() { ["type"] = "formula", ["text"] = text };

    private static JsonObject CodeNode(string text, string? language)
    {
        var node = new JsonObject { ["type"] = "code", ["text"] = text };
        if (language is not null) node["language"] = language;
        return node;
    }

    private static JsonObject ListNode(OpenList list) => new()
    {
        ["type"] = "list",
        ["ordered"] = list.Ordered,
        ["items"] = list.Items,
    };

    private static JsonObject BlockquoteNode(JsonArray body) => new() { ["type"] = "blockquote", ["body"] = body };

    private static JsonObject ImageNode(string? alt, string? src)
    {
        var node = new JsonObject { ["type"] = "image" };
        if (alt is not null) node["alt"] = alt;
        if (src is not null) node["src"] = src;
        return node;
    }

    private static JsonObject TableNode(Table table)
    {
        var headers = new JsonArray();
        var rows = new JsonArray();
        if (table.Cells.Count > 0)
        {
            foreach (var h in table.Cells[0]) headers.Add(h);
            for (int r = 1; r < table.Cells.Count; r++)
            {
                var rowArr = new JsonArray();
                foreach (var c in table.Cells[r]) rowArr.Add(c);
                rows.Add(rowArr);
            }
        }
        // caption is always None in the port.
        return new JsonObject { ["type"] = "table", ["headers"] = headers, ["rows"] = rows };
    }
}
