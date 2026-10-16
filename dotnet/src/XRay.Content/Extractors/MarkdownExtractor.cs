using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using XRay.Content.Core;
using XRay.Content.Internal.Commonmark;
using XRay.Content.Internal.Markup;
using XRay.Content.Internal.Yaml;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Markdown extractor with YAML frontmatter support. Ported from
/// <c>crates/xberg/src/extractors/markdown.rs</c> (+ <c>frontmatter_utils.rs</c>,
/// <c>annotation_utils.rs</c>).
///
/// Uses <see cref="MarkdownParser"/> (a pragmatic CommonMark/GFM parser) in place of Rust's
/// pulldown-cmark; block structure, tables and frontmatter metadata match closely, though the
/// parser is not a byte-exact pulldown-cmark clone for adversarial inline edge cases.
/// </summary>
public sealed class MarkdownExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[]
    {
        "text/markdown",
        "text/x-markdown",
        "text/x-gfm",
        "text/x-commonmark",
        "text/x-markdown-extra",
        "text/x-multimarkdown",
        "text/x-pandoc",
        "text/x-quarto",
        // application/x-quarto is an alias of text/x-quarto in the format table, so
        // validation accepts it — but the registry resolves extractors by exact string with
        // no alias resolution, so an unclaimed alias would reach extraction and fail as an
        // unsupported format despite being advertised as supported.
        "application/x-quarto",
        "text/x-r-markdown",
    };

    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        // Upstream's one-line growth guard: the input's own size is charged against
        // `max_content_size` before parsing, so a document too large to render is refused
        // rather than rendered and then found to be too large.
        SecurityBudget.FromConfig(config).AccountText(content.Length);
        string text = Encoding.UTF8.GetString(content);
        var (yaml, remaining) = ExtractFrontmatter(text);

        var metadata = yaml is not null ? ExtractMetadataFromYaml(yaml) : new Metadata();
        if (metadata.Title is null)
        {
            string? title = ExtractTitleFromContent(remaining);
            if (title is not null) metadata.Title = title;
        }

        var events = MarkdownParser.Parse(remaining);
        var doc = BuildInternalDocument(events, yaml);
        doc.Metadata = metadata;
        doc.MimeType = mimeType;
        return doc;
    }

    // ------------------------------------------------------------------
    // build_internal_document (port of markdown.rs)
    // ------------------------------------------------------------------

    internal static InternalDocument BuildInternalDocument(List<MdEvent> events, JsonNode? yaml,
        string sourceFormat = "markdown", IReadOnlyList<string>? rawJsxBlocks = null)
    {
        var b = new InternalDocumentBuilder(sourceFormat);

        // Frontmatter as a metadata block.
        if (yaml is JsonObject map)
        {
            var entries = new List<(string, string)>();
            foreach (var kv in map)
            {
                string key = kv.Key;
                string val;
                if (kv.Value is JsonValue jv && jv.TryGetValue<string>(out var s)) val = s;
                else val = YamlValueDebug(kv.Value);
                entries.Add((key, val));
            }
            if (entries.Count > 0) b.PushMetadataBlock(entries, null);
        }

        // MDX: stripped JSX components emitted as raw blocks.
        if (rawJsxBlocks is not null)
            foreach (var jsx in rawJsxBlocks)
                if (jsx.Trim().Length > 0) b.PushRawBlock("jsx", jsx, null);

        var paragraphText = new StringBuilder();
        var paragraphAnns = new List<TextAnnotation>();
        bool inParagraph = false;
        var headingText = new StringBuilder();
        var headingAnns = new List<TextAnnotation>();
        byte headingLevel = 0;
        bool inHeading = false;
        var codeText = new StringBuilder();
        string? codeLang = null;
        bool inCodeBlock = false;
        var tableRows = new List<List<string>>();
        var currentRow = new List<string>();
        var currentCell = new StringBuilder();
        bool inTableCell = false;
        var listStack = new List<bool>();
        var listItemText = new StringBuilder();
        var listItemAnns = new List<TextAnnotation>();
        bool inListItem = false;
        bool inImage = false;
        var imageAlt = new StringBuilder();
        string? imageUrl = null;
        uint imageCounter = 0;
        string? footnoteDefLabel = null;
        var footnoteDefText = new StringBuilder();
        bool inDefTitle = false;
        bool inDefDesc = false;
        var defBuf = new StringBuilder();
        var blockQuoteStack = new List<bool>();

        // annotation_starts: (kind 0=bold 1=italic 2=strike 4=link, byteStart, linkUrl, linkTitle)
        var annStarts = new List<(int kind, uint start, string? url, string? title)>();

        uint Off(StringBuilder buf) => (uint)Encoding.UTF8.GetByteCount(buf.ToString());

        int eventCounter = 0;
        foreach (var e in events)
        {
            Cancel.Every(eventCounter++, 0xFF);
            switch (e.Kind)
            {
                case MdEventKind.StartHeading:
                    headingText.Clear(); headingAnns.Clear(); annStarts.Clear();
                    headingLevel = e.Level;
                    inHeading = true;
                    break;
                case MdEventKind.EndHeading:
                {
                    inHeading = false;
                    string raw = headingText.ToString();
                    string trimmed = raw.Trim();
                    if (trimmed.Length > 0)
                    {
                        var anns = AdjustAnnotationsForTrim(headingAnns, raw, trimmed);
                        uint idx = b.PushHeading(headingLevel, trimmed, null, null);
                        if (anns.Count > 0) b.SetAnnotations(idx, anns);
                    }
                    headingText.Clear(); headingAnns.Clear();
                    break;
                }
                case MdEventKind.StartParagraph:
                    if (!inHeading && !inListItem && footnoteDefLabel is null && !inDefDesc)
                    {
                        paragraphText.Clear(); paragraphAnns.Clear();
                        inParagraph = true;
                    }
                    break;
                case MdEventKind.EndParagraph:
                    if (inParagraph)
                    {
                        inParagraph = false;
                        string raw = paragraphText.ToString();
                        string trimmed = raw.Trim();
                        if (trimmed.Length > 0)
                        {
                            var anns = AdjustAnnotationsForTrim(paragraphAnns, raw, trimmed);
                            b.PushParagraph(trimmed, anns, null, null);
                        }
                        paragraphText.Clear(); paragraphAnns.Clear();
                    }
                    break;

                case MdEventKind.StartStrong:
                    PushAnnStart(annStarts, 0, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case MdEventKind.EndStrong:
                    EndAnn(annStarts, 0, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, MakeSimple(AnnotationKind.Tag.Bold));
                    break;
                case MdEventKind.StartEmphasis:
                    PushAnnStart(annStarts, 1, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case MdEventKind.EndEmphasis:
                    EndAnn(annStarts, 1, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, MakeSimple(AnnotationKind.Tag.Italic));
                    break;
                case MdEventKind.StartStrikethrough:
                    PushAnnStart(annStarts, 2, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case MdEventKind.EndStrikethrough:
                    EndAnn(annStarts, 2, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, MakeSimple(AnnotationKind.Tag.Strikethrough));
                    break;

                // Pandoc's `^x^` and `~x~`. Like emphasis, the markers are structure rather than
                // content: they become annotations and leave the text alone.
                case MdEventKind.StartSuperscript:
                    PushAnnStart(annStarts, 5, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case MdEventKind.EndSuperscript:
                    EndAnn(annStarts, 5, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, MakeSimple(AnnotationKind.Tag.Superscript));
                    break;
                case MdEventKind.StartSubscript:
                    PushAnnStart(annStarts, 6, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case MdEventKind.EndSubscript:
                    EndAnn(annStarts, 6, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, MakeSimple(AnnotationKind.Tag.Subscript));
                    break;

                case MdEventKind.StartLink:
                {
                    string? titleOpt = string.IsNullOrEmpty(e.LinkTitle) ? null : e.LinkTitle;
                    if (inParagraph) annStarts.Add((4, Off(paragraphText), e.Url, titleOpt));
                    else if (inHeading) annStarts.Add((4, Off(headingText), e.Url, titleOpt));
                    else if (inListItem) annStarts.Add((4, Off(listItemText), e.Url, titleOpt));
                    break;
                }
                case MdEventKind.EndLink:
                {
                    int pos = annStarts.FindLastIndex(a => a.kind == 4);
                    if (pos >= 0)
                    {
                        var (_, start, url, title) = annStarts[pos];
                        annStarts.RemoveAt(pos);
                        string? label = null;
                        if (url is not null)
                        {
                            var kind = new AnnotationKind { Which = AnnotationKind.Tag.Link, Url = url, Title = title };
                            if (inParagraph)
                            {
                                uint end = Off(paragraphText);
                                if (start < end)
                                {
                                    paragraphAnns.Add(new TextAnnotation { Start = start, End = end, Kind = kind });
                                    label = Slice(paragraphText, start, end);
                                }
                            }
                            else if (inHeading)
                            {
                                uint end = Off(headingText);
                                if (start < end)
                                {
                                    headingAnns.Add(new TextAnnotation { Start = start, End = end, Kind = kind });
                                    label = Slice(headingText, start, end);
                                }
                            }
                            else if (inListItem)
                            {
                                uint end = Off(listItemText);
                                if (start < end)
                                {
                                    listItemAnns.Add(new TextAnnotation { Start = start, End = end, Kind = kind });
                                    label = Slice(listItemText, start, end);
                                }
                            }
                            if (url.Length > 0)
                                b.PushUri(new ExtractedUri
                                {
                                    Url = url,
                                    Label = string.IsNullOrEmpty(label) ? null : label,
                                    Kind = ClassifyUri(url),
                                });
                        }
                    }
                    break;
                }

                case MdEventKind.StartCodeBlock:
                    codeText.Clear();
                    codeLang = NormalizeFenceLang(e.Text);
                    inCodeBlock = true;
                    break;
                case MdEventKind.EndCodeBlock:
                {
                    inCodeBlock = false;
                    string trimmed = codeText.ToString().TrimEnd();
                    if (trimmed.Length > 0) b.PushCode(trimmed, codeLang, null, null);
                    codeText.Clear(); codeLang = null;
                    break;
                }

                // A GFM alert is an admonition, not a quote: it carries no quote container at
                // all, so its body renders as ordinary blocks alongside the kind marker.
                case MdEventKind.StartBlockQuote:
                    if (e.Text.Length > 0) { b.PushAdmonition(e.Text, null, null); blockQuoteStack.Add(true); }
                    else { b.PushQuoteStart(); blockQuoteStack.Add(false); }
                    break;
                case MdEventKind.EndBlockQuote:
                {
                    bool wasAlert = blockQuoteStack.Count > 0 && blockQuoteStack[^1];
                    if (blockQuoteStack.Count > 0) blockQuoteStack.RemoveAt(blockQuoteStack.Count - 1);
                    if (!wasAlert) b.PushQuoteEnd();
                    break;
                }

                case MdEventKind.StartList:
                    b.PushList(e.Ordered);
                    listStack.Add(e.Ordered);
                    break;
                case MdEventKind.EndList:
                    if (listStack.Count > 0)
                    {
                        listStack.RemoveAt(listStack.Count - 1);
                        b.EndList();
                    }
                    break;
                case MdEventKind.StartItem:
                    listItemText.Clear(); listItemAnns.Clear(); annStarts.Clear();
                    inListItem = true;
                    break;
                case MdEventKind.EndItem:
                {
                    inListItem = false;
                    string raw = listItemText.ToString();
                    string trimmed = raw.Trim();
                    if (listStack.Count > 0 && trimmed.Length > 0)
                    {
                        bool ordered = listStack[^1];
                        var anns = AdjustAnnotationsForTrim(listItemAnns, raw, trimmed);
                        b.PushListItem(trimmed, ordered, anns, null, null);
                    }
                    listItemText.Clear(); listItemAnns.Clear();
                    break;
                }

                case MdEventKind.StartTable: tableRows.Clear(); break;
                case MdEventKind.EndTable:
                    if (tableRows.Count > 0)
                    {
                        string md = CellsToMarkdown(tableRows);
                        var table = new Table { Cells = tableRows, Markdown = md, PageNumber = 1 };
                        b.PushTable(table, null, null);
                        tableRows = new List<List<string>>();
                    }
                    else tableRows = new List<List<string>>();
                    break;
                case MdEventKind.StartTableRow: currentRow = new List<string>(); break;
                case MdEventKind.EndTableRow:
                    if (currentRow.Count > 0) { tableRows.Add(currentRow); currentRow = new List<string>(); }
                    break;
                case MdEventKind.StartTableCell: currentCell.Clear(); inTableCell = true; break;
                case MdEventKind.EndTableCell:
                    inTableCell = false;
                    currentRow.Add(currentCell.ToString().Trim());
                    currentCell.Clear();
                    break;

                case MdEventKind.StartImage:
                    inImage = true; imageAlt.Clear(); imageUrl = e.Url;
                    break;
                case MdEventKind.EndImage:
                {
                    inImage = false;
                    string trimmed = imageAlt.ToString().Trim();
                    string? desc = trimmed.Length == 0 ? null : trimmed;
                    string? url = imageUrl is { Length: > 0 } ? imageUrl : null;

                    // Data-URI images are decoded so the emitted `Image` element's index actually
                    // resolves in `doc.Images`. Plain-URL images have no bytes to attach, and an
                    // element with an unresolvable index is silently dropped by every renderer, so
                    // their reference is preserved as visible text instead (Rust `markdown.rs`).
                    var decodedImage = url is not null && url.StartsWith("data:image/", StringComparison.Ordinal)
                        ? MarkdownUtils.DecodeDataUriImage(url, imageCounter)
                        : null;

                    if (decodedImage is not null)
                    {
                        imageCounter++;
                        decodedImage.Description = desc;
                        b.PushImage(desc, decodedImage, null, null);
                    }
                    else
                    {
                        string display = (url, desc) switch
                        {
                            (not null, not null) => $"[Image: {desc} ({url})]",
                            (not null, null) => $"[Image: {url}]",
                            (null, not null) => $"[Image: {desc}]",
                            _ => "",
                        };
                        if (display.Length > 0) b.PushParagraph(display, new(), null, null);
                    }

                    if (url is not null)
                        b.PushUri(new ExtractedUri
                        {
                            Url = url,
                            Label = desc,
                            Kind = UriKind.Image,
                        });
                    imageUrl = null;
                    imageAlt.Clear();
                    break;
                }

                case MdEventKind.StartFootnoteDefinition:
                    footnoteDefLabel = e.Text;
                    footnoteDefText.Clear();
                    break;
                case MdEventKind.EndFootnoteDefinition:
                    if (footnoteDefLabel is not null)
                    {
                        string t = footnoteDefText.ToString().Trim();
                        if (t.Length > 0) b.PushFootnoteDefinition(t, footnoteDefLabel, null);
                        footnoteDefLabel = null;
                    }
                    footnoteDefText.Clear();
                    break;

                case MdEventKind.Code:
                    if (inCodeBlock) codeText.Append(e.Text);
                    else if (inHeading) AppendCode(headingText, headingAnns, e.Text);
                    else if (inImage) imageAlt.Append(e.Text);
                    else if (inTableCell) currentCell.Append(e.Text);
                    else if (inListItem) AppendCode(listItemText, listItemAnns, e.Text);
                    else if (footnoteDefLabel is not null) footnoteDefText.Append(e.Text);
                    else if (inDefTitle || inDefDesc) defBuf.Append(e.Text);
                    else if (inParagraph) AppendCode(paragraphText, paragraphAnns, e.Text);
                    break;
                // Inline math stays in the text with its delimiters, since `$x$` reads as maths
                // wherever the text ends up. Display math is a block of its own and becomes a
                // Formula element, delimiters dropped.
                case MdEventKind.InlineMath:
                    if (inHeading) headingText.Append('$').Append(e.Text).Append('$');
                    else if (inTableCell) currentCell.Append('$').Append(e.Text).Append('$');
                    else if (inListItem) listItemText.Append('$').Append(e.Text).Append('$');
                    else if (footnoteDefLabel is not null) footnoteDefText.Append('$').Append(e.Text).Append('$');
                    else if (inDefTitle || inDefDesc) defBuf.Append('$').Append(e.Text).Append('$');
                    else if (inParagraph) paragraphText.Append('$').Append(e.Text).Append('$');
                    break;
                case MdEventKind.DisplayMath:
                {
                    string formula = e.Text.Trim();
                    if (formula.Length > 0) b.PushFormula(formula, null, null);
                    break;
                }
                case MdEventKind.Text:
                    if (inCodeBlock) codeText.Append(e.Text);
                    else if (inHeading) headingText.Append(e.Text);
                    else if (inImage) imageAlt.Append(e.Text);
                    else if (inTableCell) currentCell.Append(e.Text);
                    else if (inListItem) listItemText.Append(e.Text);
                    else if (footnoteDefLabel is not null) footnoteDefText.Append(e.Text);
                    else if (inDefTitle || inDefDesc) defBuf.Append(e.Text);
                    else if (inParagraph) paragraphText.Append(e.Text);
                    break;
                case MdEventKind.SoftBreak:
                case MdEventKind.HardBreak:
                    if (inCodeBlock) codeText.Append('\n');
                    else if (inHeading) headingText.Append(' ');
                    else if (inListItem) listItemText.Append(' ');
                    else if (footnoteDefLabel is not null) footnoteDefText.Append(' ');
                    else if (inDefTitle || inDefDesc) defBuf.Append(' ');
                    else if (inParagraph) paragraphText.Append(' ');
                    break;
                case MdEventKind.FootnoteReference:
                    b.PushFootnoteRef(e.Text, e.Text, null);
                    break;
                // Raw HTML goes into whichever block is currently open. Block-level HTML (e.g. a
                // bare `<div>…</div>` or an `<!-- image -->` comment between blank lines) arrives
                // with no block open at all; record it as a raw block instead of dropping it.
                case MdEventKind.Html:
                    if (inHeading) headingText.Append(e.Text);
                    else if (inTableCell) currentCell.Append(e.Text);
                    else if (inListItem) listItemText.Append(e.Text);
                    else if (footnoteDefLabel is not null) footnoteDefText.Append(e.Text);
                    else if (inDefTitle || inDefDesc) defBuf.Append(e.Text);
                    else if (inParagraph) paragraphText.Append(e.Text);
                    else
                    {
                        string trimmedHtml = e.Text.Trim();
                        if (trimmedHtml.Length > 0) b.PushRawBlock("html", trimmedHtml, null);
                    }
                    break;
                case MdEventKind.TaskListMarker:
                    if (inListItem) listItemText.Append(e.Checked ? "[x] " : "[ ] ");
                    break;

                case MdEventKind.StartDefinitionListTitle:
                    inDefTitle = true; defBuf.Clear();
                    break;
                case MdEventKind.EndDefinitionListTitle:
                {
                    inDefTitle = false;
                    string term = defBuf.ToString().Trim();
                    if (term.Length > 0) b.PushDefinitionTerm(term, null);
                    defBuf.Clear();
                    break;
                }
                case MdEventKind.StartDefinitionListDefinition:
                    inDefDesc = true; defBuf.Clear();
                    break;
                case MdEventKind.EndDefinitionListDefinition:
                {
                    inDefDesc = false;
                    string desc = defBuf.ToString().Trim();
                    if (desc.Length > 0) b.PushDefinitionDescription(desc, null);
                    defBuf.Clear();
                    break;
                }
            }
        }

        return b.Build();

        static AnnotationKind MakeSimple(AnnotationKind.Tag tag) => new() { Which = tag };
    }

    /// <summary>
    /// The language a fence's info string names.
    /// </summary>
    /// <remarks>
    /// An info string may carry more than the language: renderer options after a space or comma
    /// (<c>```mdx-invalid chrome=no</c>), Pandoc-style braces and a leading dot
    /// (<c>```{.python}</c>). Only the first token is the language; passing the whole string
    /// through puts the options into the rendered fence.
    /// </remarks>
    private static string? NormalizeFenceLang(string? info)
    {
        string trimmed = (info ?? "").Trim();
        if (trimmed.Length == 0) return null;

        string inner = trimmed;
        if (inner.StartsWith('{'))
        {
            inner = inner[1..];
            if (inner.EndsWith('}')) inner = inner[..^1];
        }

        string lang = inner.Split(',', ' ', '\t')[0].Trim().TrimStart('.');
        return lang.Length == 0 ? null : lang;
    }

    /// <summary>Renders a non-string YAML frontmatter value the way the Rust extractor does —
    /// <c>format!("{value:?}")</c> on a <c>serde_yaml</c> <c>Value</c>. serde_yaml implements a
    /// custom Debug: <c>Sequence [..]</c>, <c>String("..")</c>, <c>Number(n)</c>, <c>Bool(b)</c>,
    /// <c>Null</c>, and mappings as <c>{k: v}</c>.</summary>
    private static string YamlValueDebug(JsonNode? node)
    {
        switch (node)
        {
            case null:
                return "Null";
            case JsonArray arr:
            {
                var items = arr.Select(YamlValueDebug);
                return "Sequence [" + string.Join(", ", items) + "]";
            }
            case JsonObject obj:
            {
                var kvs = obj.Select(kv => $"{YamlDebugString(kv.Key)}: {YamlValueDebug(kv.Value)}");
                return "Mapping {" + string.Join(", ", kvs) + "}";
            }
            case JsonValue v:
            {
                if (v.TryGetValue<string>(out var s)) return $"String({YamlDebugString(s)})";
                if (v.TryGetValue<bool>(out var b)) return b ? "Bool(true)" : "Bool(false)";
                return $"Number({v.ToJsonString()})";
            }
            default:
                return node.ToJsonString();
        }
    }

    /// <summary>Rust's <c>{:?}</c> string formatting: double-quoted with C-style escapes.</summary>
    private static string YamlDebugString(string s)
    {
        var sb = new StringBuilder(s.Length + 2);
        sb.Append('"');
        foreach (char c in s)
        {
            switch (c)
            {
                case '"': sb.Append("\\\""); break;
                case '\\': sb.Append("\\\\"); break;
                case '\n': sb.Append("\\n"); break;
                case '\t': sb.Append("\\t"); break;
                case '\r': sb.Append("\\r"); break;
                default: sb.Append(c); break;
            }
        }
        sb.Append('"');
        return sb.ToString();
    }

    private static void PushAnnStart(List<(int, uint, string?, string?)> annStarts, int kind,
        bool inParagraph, bool inHeading, bool inListItem,
        StringBuilder para, StringBuilder heading, StringBuilder item)
    {
        if (inParagraph) annStarts.Add((kind, (uint)Encoding.UTF8.GetByteCount(para.ToString()), null, null));
        else if (inHeading) annStarts.Add((kind, (uint)Encoding.UTF8.GetByteCount(heading.ToString()), null, null));
        else if (inListItem) annStarts.Add((kind, (uint)Encoding.UTF8.GetByteCount(item.ToString()), null, null));
    }

    private static void EndAnn(List<(int kind, uint start, string? url, string? title)> annStarts, int kind,
        bool inParagraph, bool inHeading, bool inListItem,
        StringBuilder para, StringBuilder heading, StringBuilder item,
        List<TextAnnotation> paraAnns, List<TextAnnotation> headingAnns, List<TextAnnotation> itemAnns,
        AnnotationKind annKind)
    {
        int pos = annStarts.FindLastIndex(a => a.kind == kind);
        if (pos < 0) return;
        uint start = annStarts[pos].start;
        annStarts.RemoveAt(pos);
        if (inParagraph)
        {
            uint end = (uint)Encoding.UTF8.GetByteCount(para.ToString());
            if (start < end) paraAnns.Add(new TextAnnotation { Start = start, End = end, Kind = annKind });
        }
        else if (inHeading)
        {
            uint end = (uint)Encoding.UTF8.GetByteCount(heading.ToString());
            if (start < end) headingAnns.Add(new TextAnnotation { Start = start, End = end, Kind = annKind });
        }
        else if (inListItem)
        {
            uint end = (uint)Encoding.UTF8.GetByteCount(item.ToString());
            if (start < end) itemAnns.Add(new TextAnnotation { Start = start, End = end, Kind = annKind });
        }
    }

    private static void AppendCode(StringBuilder buf, List<TextAnnotation> anns, string s)
    {
        uint start = (uint)Encoding.UTF8.GetByteCount(buf.ToString());
        buf.Append(s);
        uint end = (uint)Encoding.UTF8.GetByteCount(buf.ToString());
        if (start < end) anns.Add(new TextAnnotation { Start = start, End = end, Kind = new AnnotationKind { Which = AnnotationKind.Tag.Code } });
    }

    private static string Slice(StringBuilder buf, uint start, uint end)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(buf.ToString());
        return Encoding.UTF8.GetString(bytes, (int)start, (int)(end - start));
    }

    // ------------------------------------------------------------------
    // annotation_utils / frontmatter_utils ports
    // ------------------------------------------------------------------

    internal static List<TextAnnotation> AdjustAnnotationsForTrim(List<TextAnnotation> annotations, string raw, string trimmed)
    {
        uint offset = (uint)(Encoding.UTF8.GetByteCount(raw) - Encoding.UTF8.GetByteCount(raw.TrimStart()));
        uint trimmedLen = (uint)Encoding.UTF8.GetByteCount(trimmed);
        // Offsets shift left by the leading whitespace removed and are then *clamped* to the
        // trimmed length. A span that runs into the trailing whitespace still covers real words;
        // dropping it outright loses the formatting (upstream's #226). Only a span that collapses
        // to an empty range — one that lay entirely inside the whitespace — is discarded.
        return annotations.Select(a => new TextAnnotation
        {
            Start = Math.Min(a.Start >= offset ? a.Start - offset : 0, trimmedLen),
            End = Math.Min(a.End >= offset ? a.End - offset : 0, trimmedLen),
            Kind = a.Kind,
        }).Where(a => a.Start < a.End).ToList();
    }

    internal static (JsonNode? Yaml, string Remaining) ExtractFrontmatter(string content)
    {
        if (!content.StartsWith("---", StringComparison.Ordinal)) return (null, content);
        string rest = content.Substring(3);

        int? endPos = null;
        int searchStart = 0;
        while (true)
        {
            int pos = rest.IndexOf('\n', searchStart);
            if (pos < 0) break;
            int afterNewline = pos + 1;
            if (afterNewline >= rest.Length) break;
            string remaining = rest.Substring(afterNewline);
            if (remaining.StartsWith("---", StringComparison.Ordinal) || remaining.StartsWith("...", StringComparison.Ordinal))
            {
                int delimiterEnd = afterNewline + 3;
                if (delimiterEnd >= rest.Length || rest[delimiterEnd] == '\n')
                {
                    endPos = pos;
                    break;
                }
            }
            searchStart = afterNewline;
        }

        if (endPos is not int end) return (null, content);

        string frontmatterStr = rest.Substring(0, end);
        int afterDelimiter = end + 1;
        int remainingStart;
        if (afterDelimiter + 3 < rest.Length)
        {
            int afterDelim = afterDelimiter + 3;
            remainingStart = (afterDelim < rest.Length && rest[afterDelim] == '\n') ? afterDelim + 1 : afterDelim;
        }
        else remainingStart = rest.Length;

        string remainingContent = remainingStart < rest.Length ? rest.Substring(remainingStart) : "";

        var yaml = YamlParser.Parse(frontmatterStr);
        if (yaml is null) return (null, content);
        return (yaml, remainingContent);
    }

    internal static Metadata ExtractMetadataFromYaml(JsonNode yaml)
    {
        var m = new Metadata();
        string? Str(string key) => yaml[key] is JsonValue v && v.TryGetValue<string>(out var s) ? s : null;

        if (Str("title") is string title && m.Title is null) m.Title = title;

        // `author` may be a scalar or a sequence, and the Hugo/Jekyll `authors` key is the same
        // shape; `Metadata.Authors` is the typed home for all three. Where both keys are present
        // `authors` wins for the list, while a scalar `author` still supplies `CreatedBy`.
        if (yaml["author"] is JsonNode authorValue)
        {
            if (Str("author") is string author && m.CreatedBy is null) m.CreatedBy = author;
            m.Authors ??= ScalarOrSequence(authorValue);
        }
        if (yaml["authors"] is JsonNode authorsValue && ScalarOrSequence(authorsValue) is { } authors)
            m.Authors = authors;

        if (Str("date") is string date) m.CreatedAt = date;

        if (yaml["keywords"] is JsonNode kw)
        {
            if (kw is JsonValue kv && kv.TryGetValue<string>(out var ks) && m.Keywords is null)
                m.Keywords = ks.Split(',').Select(x => x.Trim()).ToList();
            else if (kw is JsonArray ka && m.Keywords is null)
                m.Keywords = ka.Where(x => x is JsonValue jv && jv.TryGetValue<string>(out _))
                    .Select(x => x!.GetValue<string>()).ToList();
        }

        if (Str("description") is string desc) m.Subject = desc;
        if (Str("abstract") is string abs) m.AbstractText = abs;
        if (Str("subject") is string subj) m.Subject = subj;
        if (Str("category") is string cat) m.Category = cat;

        if (yaml["tags"] is JsonNode tg)
        {
            if (tg is JsonValue tv && tv.TryGetValue<string>(out var ts))
                m.Tags = ts.Split(',').Select(x => x.Trim()).ToList();
            else if (tg is JsonArray ta)
                m.Tags = ta.Where(x => x is JsonValue jv && jv.TryGetValue<string>(out _))
                    .Select(x => x!.GetValue<string>()).ToList();
        }

        if (Str("language") is string lang && m.Language is null) m.Language = lang;
        if (Str("version") is string ver) m.DocumentVersion = ver;

        // Every top-level key without a typed field above (Hugo/Jekyll/Obsidian extras like
        // `aliases`, `slug`, `draft`, or anything a document invents) is preserved rather than
        // dropped, keeping its YAML shape: a sequence stays an array, not a joined string.
        if (yaml is JsonObject obj)
        {
            foreach (var kv in obj)
            {
                if (KnownFrontmatterKeys.Contains(kv.Key)) continue;
                m.Additional[kv.Key] = JsonSerializer.SerializeToElement(kv.Value);
            }
        }
        return m;
    }

    /// <summary>Frontmatter keys that have a typed home on <see cref="Metadata"/>; everything
    /// else lands in <c>additional</c>.</summary>
    private static readonly HashSet<string> KnownFrontmatterKeys = new(StringComparer.Ordinal)
    {
        "title", "author", "authors", "date", "keywords", "description", "abstract",
        "subject", "category", "tags", "language", "version",
    };

    /// <summary>A frontmatter value read as a list of names: one entry for a scalar, its string
    /// members for a sequence, and null for anything else or an empty sequence
    /// (<c>yaml_scalar_or_sequence_to_strings</c>).</summary>
    private static List<string>? ScalarOrSequence(JsonNode value)
    {
        if (value is JsonValue v && v.TryGetValue<string>(out var s)) return new List<string> { s };
        if (value is JsonArray a)
        {
            var items = a.Where(x => x is JsonValue jv && jv.TryGetValue<string>(out _))
                .Select(x => x!.GetValue<string>()).ToList();
            return items.Count > 0 ? items : null;
        }
        return null;
    }

    internal static string? ExtractTitleFromContent(string content)
    {
        foreach (var line in RenderCommonLines(content))
        {
            if (line.StartsWith("# ", StringComparison.Ordinal))
                return line.Substring(2).Trim();
        }
        return null;
    }

    private static IEnumerable<string> RenderCommonLines(string text)
    {
        int start = 0;
        for (int i = 0; i < text.Length; i++)
        {
            if (text[i] == '\n')
            {
                int end = i;
                if (end > start && text[end - 1] == '\r') end--;
                yield return text.Substring(start, end - start);
                start = i + 1;
            }
        }
        if (start < text.Length)
        {
            int end = text.Length;
            if (end > start && text[end - 1] == '\r') end--;
            yield return text.Substring(start, end - start);
        }
    }

    internal static string CellsToMarkdown(List<List<string>> cells)
    {
        if (cells.Count == 0) return "";
        var md = new StringBuilder();
        md.Append('|');
        foreach (var cell in cells[0]) { md.Append(' '); md.Append(cell); md.Append(" |"); }
        md.Append('\n');
        md.Append('|');
        foreach (var _ in cells[0]) md.Append(" --- |");
        md.Append('\n');
        for (int r = 1; r < cells.Count; r++)
        {
            md.Append('|');
            foreach (var cell in cells[r]) { md.Append(' '); md.Append(cell); md.Append(" |"); }
            md.Append('\n');
        }
        return md.ToString();
    }

    private static UriKind ClassifyUri(string url)
    {
        if (url.StartsWith("mailto:", StringComparison.OrdinalIgnoreCase)) return UriKind.Email;
        if (url.StartsWith("#", StringComparison.Ordinal)) return UriKind.Anchor;
        return UriKind.Hyperlink;
    }
}
