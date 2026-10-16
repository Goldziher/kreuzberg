using System.Text;
using System.Text.Json.Nodes;
using XRay.Content.Core;
using XRay.Content.Internal.Djot;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Djot markup extractor with YAML frontmatter and table support. Ported from
/// <c>crates/xberg/src/extractors/djot_format/extractor.rs</c> and
/// <c>parsing/table_extraction.rs</c>.
///
/// Uses <see cref="DjotParser"/> (a pragmatic Djot parser) in place of Rust's <c>jotdown</c>
/// crate: it emits a jotdown-like event stream that this extractor walks to build the
/// <see cref="InternalDocument"/>. Frontmatter/metadata/annotation-trim/table-markdown helpers
/// are shared verbatim with <see cref="MarkdownExtractor"/> (both mirror the same Rust
/// <c>frontmatter_utils</c> / <c>annotation_utils</c>).
/// </summary>
public sealed class DjotExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[]
    {
        "text/djot",
        "text/x-djot",
    };

    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        string text = Encoding.UTF8.GetString(content);
        var (yaml, remaining) = MarkdownExtractor.ExtractFrontmatter(text);

        var metadata = yaml is not null ? MarkdownExtractor.ExtractMetadataFromYaml(yaml) : new Metadata();
        if (metadata.Title is null)
        {
            string? title = MarkdownExtractor.ExtractTitleFromContent(remaining);
            if (title is not null) metadata.Title = title;
        }

        var events = DjotParser.Parse(remaining);

        // Tables are parsed in place, in the same pass as everything else, so each one keeps its
        // position in the document and gets a Table *element*. Recording the table data alone —
        // which is what a separate pass and a bare `PushTable` did — left every renderer with
        // nothing to walk, and the table's content silently vanished from the output.
        var doc = BuildInternalDocument(events);
        doc.MimeType = mimeType;
        doc.Metadata = metadata;

        return doc;
    }

    // ------------------------------------------------------------------
    // build_internal_document (port of extractor.rs)
    // ------------------------------------------------------------------

    internal static InternalDocument BuildInternalDocument(List<DjotEvent> events)
    {
        var b = new InternalDocumentBuilder("djot");

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
        bool inMath = false;
        var mathText = new StringBuilder();
        var listStack = new List<bool>();
        var listItemText = new StringBuilder();
        var listItemAnns = new List<TextAnnotation>();
        bool inListItem = false;
        bool inRawBlock = false;
        string? rawFormat = null;
        var rawText = new StringBuilder();
        bool inVerbatim = false;
        uint verbatimStart = 0;
        bool inImage = false;
        var imageAlt = new StringBuilder();
        bool inFootnote = false;
        string footnoteLabel = "";
        var footnoteText = new StringBuilder();
        List<List<string>>? tableRows = null;
        var tableRow = new List<string>();
        var tableCell = new StringBuilder();
        bool inTableCell = false;

        // Annotation tracking: (kindTag 0=strong 1=emphasis 2=delete 4=link, byteStart, linkUrl).
        var annStarts = new List<(int kind, uint start, string? url)>();

        static uint Off(StringBuilder buf) => (uint)Encoding.UTF8.GetByteCount(buf.ToString());

        int eventCounter = 0;
        foreach (var e in events)
        {
            Cancel.Every(eventCounter++, 0xFF);
            switch (e.Kind)
            {
                case DjotEventKind.StartHeading:
                    headingText.Clear(); headingAnns.Clear(); annStarts.Clear();
                    headingLevel = e.Level;
                    inHeading = true;
                    break;
                case DjotEventKind.EndHeading:
                {
                    inHeading = false;
                    string raw = headingText.ToString();
                    string trimmed = raw.Trim();
                    if (trimmed.Length > 0)
                    {
                        var anns = MarkdownExtractor.AdjustAnnotationsForTrim(headingAnns, raw, trimmed);
                        uint idx = b.PushHeading(headingLevel, trimmed, null, null);
                        if (anns.Count > 0) b.SetAnnotations(idx, anns);
                    }
                    headingText.Clear(); headingAnns.Clear();
                    break;
                }

                case DjotEventKind.StartParagraph:
                    if (!inHeading && !inListItem)
                    {
                        paragraphText.Clear(); paragraphAnns.Clear();
                        inParagraph = true;
                    }
                    break;
                case DjotEventKind.EndParagraph:
                    if (inParagraph)
                    {
                        inParagraph = false;
                        string raw = paragraphText.ToString();
                        string trimmed = raw.Trim();
                        if (trimmed.Length > 0)
                        {
                            var anns = MarkdownExtractor.AdjustAnnotationsForTrim(paragraphAnns, raw, trimmed);
                            b.PushParagraph(trimmed, anns, null, null);
                        }
                        paragraphText.Clear(); paragraphAnns.Clear();
                    }
                    break;

                case DjotEventKind.StartStrong:
                    PushAnnStart(annStarts, 0, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case DjotEventKind.EndStrong:
                    EndAnn(annStarts, 0, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, AnnotationKind.Bold);
                    break;
                case DjotEventKind.StartEmphasis:
                    PushAnnStart(annStarts, 1, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case DjotEventKind.EndEmphasis:
                    EndAnn(annStarts, 1, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, AnnotationKind.Italic);
                    break;
                case DjotEventKind.StartDelete:
                    PushAnnStart(annStarts, 2, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText);
                    break;
                case DjotEventKind.EndDelete:
                    EndAnn(annStarts, 2, inParagraph, inHeading, inListItem, paragraphText, headingText, listItemText,
                        paragraphAnns, headingAnns, listItemAnns, new AnnotationKind { Which = AnnotationKind.Tag.Strikethrough });
                    break;

                case DjotEventKind.StartVerbatim:
                    if (inParagraph) { inVerbatim = true; verbatimStart = Off(paragraphText); }
                    else if (inHeading) { inVerbatim = true; verbatimStart = Off(headingText); }
                    else if (inListItem) { inVerbatim = true; verbatimStart = Off(listItemText); }
                    break;
                case DjotEventKind.EndVerbatim:
                    if (inVerbatim)
                    {
                        inVerbatim = false;
                        var codeKind = new AnnotationKind { Which = AnnotationKind.Tag.Code };
                        if (inParagraph)
                        {
                            uint end = Off(paragraphText);
                            if (verbatimStart < end) paragraphAnns.Add(new TextAnnotation { Start = verbatimStart, End = end, Kind = codeKind });
                        }
                        else if (inHeading)
                        {
                            uint end = Off(headingText);
                            if (verbatimStart < end) headingAnns.Add(new TextAnnotation { Start = verbatimStart, End = end, Kind = codeKind });
                        }
                        else if (inListItem)
                        {
                            uint end = Off(listItemText);
                            if (verbatimStart < end) listItemAnns.Add(new TextAnnotation { Start = verbatimStart, End = end, Kind = codeKind });
                        }
                    }
                    break;

                case DjotEventKind.StartLink:
                    if (inParagraph) annStarts.Add((4, Off(paragraphText), e.Url));
                    else if (inHeading) annStarts.Add((4, Off(headingText), e.Url));
                    else if (inListItem) annStarts.Add((4, Off(listItemText), e.Url));
                    break;
                case DjotEventKind.EndLink:
                {
                    int pos = annStarts.FindLastIndex(a => a.kind == 4);
                    if (pos >= 0)
                    {
                        var (_, start, url) = annStarts[pos];
                        annStarts.RemoveAt(pos);
                        if (url is not null)
                        {
                            var kind = new AnnotationKind { Which = AnnotationKind.Tag.Link, Url = url };
                            string? label = null;
                            if (inParagraph)
                            {
                                uint end = Off(paragraphText);
                                if (start < end) { paragraphAnns.Add(new TextAnnotation { Start = start, End = end, Kind = kind }); label = Slice(paragraphText, start, end); }
                            }
                            else if (inHeading)
                            {
                                uint end = Off(headingText);
                                if (start < end) { headingAnns.Add(new TextAnnotation { Start = start, End = end, Kind = kind }); label = Slice(headingText, start, end); }
                            }
                            else if (inListItem)
                            {
                                uint end = Off(listItemText);
                                if (start < end) { listItemAnns.Add(new TextAnnotation { Start = start, End = end, Kind = kind }); label = Slice(listItemText, start, end); }
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

                case DjotEventKind.StartCodeBlock:
                    codeText.Clear();
                    codeLang = string.IsNullOrEmpty(e.Text) ? null : e.Text;
                    inCodeBlock = true;
                    break;
                case DjotEventKind.EndCodeBlock:
                {
                    inCodeBlock = false;
                    string trimmed = codeText.ToString().TrimEnd();
                    if (trimmed.Length > 0) b.PushCode(trimmed, codeLang, null, null);
                    codeText.Clear(); codeLang = null;
                    break;
                }

                case DjotEventKind.StartRawBlock:
                    inRawBlock = true;
                    rawFormat = e.Text;
                    rawText.Clear();
                    break;
                case DjotEventKind.EndRawBlock:
                {
                    inRawBlock = false;
                    string trimmed = rawText.ToString().Trim();
                    if (trimmed.Length > 0) b.PushRawBlock(string.IsNullOrEmpty(rawFormat) ? "unknown" : rawFormat!, trimmed, null);
                    rawText.Clear(); rawFormat = null;
                    break;
                }

                case DjotEventKind.StartBlockquote: b.PushQuoteStart(); break;
                case DjotEventKind.EndBlockquote: b.PushQuoteEnd(); break;

                case DjotEventKind.StartList:
                    b.PushList(e.Ordered);
                    listStack.Add(e.Ordered);
                    break;
                case DjotEventKind.EndList:
                    if (listStack.Count > 0)
                    {
                        listStack.RemoveAt(listStack.Count - 1);
                        b.EndList();
                    }
                    break;
                case DjotEventKind.StartListItem:
                    listItemText.Clear(); listItemAnns.Clear(); annStarts.Clear();
                    inListItem = true;
                    break;
                case DjotEventKind.EndListItem:
                {
                    inListItem = false;
                    string raw = listItemText.ToString();
                    string trimmed = raw.Trim();
                    if (listStack.Count > 0 && trimmed.Length > 0)
                    {
                        bool ordered = listStack[^1];
                        var anns = MarkdownExtractor.AdjustAnnotationsForTrim(listItemAnns, raw, trimmed);
                        b.PushListItem(trimmed, ordered, anns, null, null);
                    }
                    listItemText.Clear(); listItemAnns.Clear();
                    break;
                }

                case DjotEventKind.StartMath when e.Display:
                    inMath = true; mathText.Clear();
                    break;
                case DjotEventKind.EndMath when e.Display:
                {
                    inMath = false;
                    string trimmed = mathText.ToString().Trim();
                    if (trimmed.Length > 0) b.PushFormula(trimmed, null, null);
                    mathText.Clear();
                    break;
                }

                case DjotEventKind.StartImage:
                    inImage = true; imageAlt.Clear();
                    break;
                case DjotEventKind.EndImage:
                {
                    inImage = false;
                    string alt = imageAlt.ToString().Trim();
                    var kind = ElementKind.Image(uint.MaxValue);
                    var elem = new InternalElement
                    {
                        Id = InternalElementId.Generate(kind.Discriminant(), alt, null, 0),
                        Kind = kind,
                        Text = alt,
                        Depth = 0,
                        Layer = ContentLayer.Body,
                    };
                    b.PushElement(elem);
                    string src = e.Url ?? "";
                    if (src.Length > 0)
                        b.PushUri(new ExtractedUri
                        {
                            Url = src,
                            Label = alt.Length == 0 ? null : alt,
                            Kind = UriKind.Image,
                        });
                    imageAlt.Clear();
                    break;
                }

                case DjotEventKind.StartFootnote:
                    inFootnote = true;
                    footnoteLabel = e.Text;
                    footnoteText.Clear();
                    break;
                case DjotEventKind.EndFootnote:
                    if (inFootnote)
                    {
                        inFootnote = false;
                        string t = footnoteText.ToString().Trim();
                        if (t.Length > 0) b.PushFootnoteDefinition(t, footnoteLabel, null);
                        footnoteText.Clear(); footnoteLabel = "";
                    }
                    break;
                case DjotEventKind.FootnoteReference:
                    b.PushFootnoteRef(e.Text, e.Text, null);
                    break;

                case DjotEventKind.StartTable:
                    tableRows = new List<List<string>>();
                    break;
                case DjotEventKind.StartTableRow:
                    tableRow = new List<string>();
                    break;
                case DjotEventKind.StartTableCell:
                    tableCell.Clear();
                    inTableCell = true;
                    break;
                case DjotEventKind.EndTableCell when inTableCell:
                    tableRow.Add(tableCell.ToString().Trim());
                    tableCell.Clear();
                    inTableCell = false;
                    break;
                case DjotEventKind.EndTableRow:
                    if (tableRow.Count > 0 && tableRows is not null) tableRows.Add(tableRow);
                    tableRow = new List<string>();
                    break;
                case DjotEventKind.EndTable:
                    if (tableRows is { Count: > 0 } rows) b.PushTableFromCells(rows, null, null);
                    tableRows = null;
                    break;
                case DjotEventKind.Str when inTableCell:
                    tableCell.Append(e.Text);
                    break;
                case DjotEventKind.Str:
                    if (inImage) imageAlt.Append(e.Text);
                    else if (inFootnote) footnoteText.Append(e.Text);
                    else if (inCodeBlock) codeText.Append(e.Text);
                    else if (inRawBlock) rawText.Append(e.Text);
                    else if (inMath) mathText.Append(e.Text);
                    else if (inHeading) headingText.Append(e.Text);
                    else if (inListItem) listItemText.Append(e.Text);
                    else if (inParagraph) paragraphText.Append(e.Text);
                    break;
                case DjotEventKind.Softbreak:
                    if (inCodeBlock) codeText.Append('\n');
                    else if (inHeading) headingText.Append(' ');
                    else if (inListItem) listItemText.Append(' ');
                    else if (inParagraph) paragraphText.Append(' ');
                    break;
                case DjotEventKind.Hardbreak:
                    if (inCodeBlock) codeText.Append('\n');
                    else if (inParagraph) paragraphText.Append('\n');
                    break;
            }
        }

        return b.Build();
    }

    // ------------------------------------------------------------------
    // Annotation helpers (mirror the Rust builder::bold/italic/... offset logic)
    // ------------------------------------------------------------------

    private static void PushAnnStart(List<(int, uint, string?)> annStarts, int kind,
        bool inParagraph, bool inHeading, bool inListItem,
        StringBuilder para, StringBuilder heading, StringBuilder item)
    {
        if (inParagraph) annStarts.Add((kind, (uint)Encoding.UTF8.GetByteCount(para.ToString()), null));
        else if (inHeading) annStarts.Add((kind, (uint)Encoding.UTF8.GetByteCount(heading.ToString()), null));
        else if (inListItem) annStarts.Add((kind, (uint)Encoding.UTF8.GetByteCount(item.ToString()), null));
    }

    private static void EndAnn(List<(int kind, uint start, string? url)> annStarts, int kind,
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

    private static string Slice(StringBuilder buf, uint start, uint end)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(buf.ToString());
        return Encoding.UTF8.GetString(bytes, (int)start, (int)(end - start));
    }

    private static UriKind ClassifyUri(string url)
    {
        if (url.StartsWith("mailto:", StringComparison.OrdinalIgnoreCase)) return UriKind.Email;
        if (url.StartsWith("#", StringComparison.Ordinal)) return UriKind.Anchor;
        return UriKind.Hyperlink;
    }
}
