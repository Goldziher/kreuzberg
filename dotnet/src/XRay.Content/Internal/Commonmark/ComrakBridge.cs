using System.Text;
using XRay.Content.Rendering;
using XRay.Content.Core.Ocr;
using XRay.Content.Types;

namespace XRay.Content.Internal.Commonmark;

/// <summary>
/// Port of <c>crates/xberg/src/rendering/comrak_bridge.rs</c>: builds a comrak-equivalent AST
/// (<see cref="MdNode"/>) from an <see cref="InternalDocument"/>. The AST is then serialized by
/// <see cref="CommonMarkFormatter"/> or <see cref="HtmlFormatter"/>.
/// </summary>
internal static class ComrakBridge
{
    private enum ContainerKind { List, BlockQuote, Group }

    private sealed class ContainerEntry
    {
        public MdNode Node = null!;
        public ContainerKind Kind;
    }

    public static MdNode Build(InternalDocument doc)
    {
        var root = new MdNode(NodeType.Document);
        var footnotes = new FootnoteCollector(doc);
        var state = new RenderState();
        var consolidated = ConsolidateParagraphs(doc.Elements);

        var containerStack = new List<ContainerEntry>();

        MdNode CurrentParent() => containerStack.Count > 0 ? containerStack[^1].Node : root;

        int consolidatedIndex = 0;
        foreach (var ce in consolidated)
        {
            Cancel.Every(consolidatedIndex++, 0x3FF);
            ElementKind elemKind;
            string elemText;
            IReadOnlyList<TextAnnotation> elemAnnotations;
            ushort elemDepth;
            Dictionary<string, string>? elemAttributes;
            int? origIdx;

            if (ce.MergedText is not null)
            {
                elemKind = ElementKind.Paragraph;
                elemText = ce.MergedText;
                elemAnnotations = ce.MergedAnnotations!;
                elemDepth = 0;
                elemAttributes = null;
                origIdx = null;
            }
            else
            {
                var elem = doc.Elements[ce.OriginalIndex];
                if (!RenderCommon.IsBodyElement(elem)) continue;
                if (RenderCommon.IsContainerEnd(elem))
                {
                    RenderCommon.HandleContainerEnd(elem.Kind, state);
                    switch (elem.Kind.Tag)
                    {
                        case ElementKindTag.ListEnd: PopContainer(containerStack, ContainerKind.List); break;
                        case ElementKindTag.QuoteEnd: PopContainer(containerStack, ContainerKind.BlockQuote); break;
                        case ElementKindTag.GroupEnd: PopContainer(containerStack, ContainerKind.Group); break;
                    }
                    continue;
                }
                state.PopToDepth(elem.Depth);
                elemKind = elem.Kind;
                elemText = elem.Text;
                elemAnnotations = elem.Annotations;
                elemDepth = elem.Depth;
                elemAttributes = elem.Attributes;
                origIdx = ce.OriginalIndex;
            }

            var parent = CurrentParent();

            // List nodes can only contain Item children; redirect non-Item blocks to the last Item.
            if (parent.Type == NodeType.List
                && elemKind.Tag != ElementKindTag.ListItem && elemKind.Tag != ElementKindTag.ListEnd)
            {
                MdNode? lastItem = null;
                foreach (var c in parent.Children())
                    if (c.Type is NodeType.Item or NodeType.TaskItem) lastItem = c;
                if (lastItem is not null) parent = lastItem;
                else
                {
                    var implicitItem = new MdNode(NodeType.Item) { List = parent.List };
                    parent.Append(implicitItem);
                    parent = implicitItem;
                }
            }

            switch (elemKind.Tag)
            {
                case ElementKindTag.Title:
                {
                    var heading = new MdNode(NodeType.Heading) { Heading = new NodeHeading { Level = 1 } };
                    BuildInlines(heading, elemText, elemAnnotations);
                    parent.Append(heading);
                    break;
                }
                case ElementKindTag.Heading:
                {
                    var heading = new MdNode(NodeType.Heading)
                    {
                        Heading = new NodeHeading { Level = Math.Clamp(elemKind.Level, MinHeadingLevel, MaxHeadingLevel) },
                    };
                    BuildInlines(heading, elemText, elemAnnotations);
                    parent.Append(heading);
                    break;
                }
                case ElementKindTag.Paragraph:
                {
                    if (elemText.Length == 0 && elemAnnotations.Count == 0) continue;
                    var para = new MdNode(NodeType.Paragraph);
                    BuildInlines(para, elemText, elemAnnotations);
                    parent.Append(para);
                    break;
                }
                case ElementKindTag.ListItem:
                {
                    // The CommonMark writer takes the marker style and the running number from the
                    // *enclosing* List node alone — an item's own NodeList is not consulted — and
                    // CommonMark cannot express a lettered or parenthesized marker like "B." or
                    // "(a)" as an ordinal at all. So when a source label survived, it is written as
                    // leading text and the list is forced to bulleted: emitting it as ordered would
                    // print a synthesized "1." beside the real "B.", renumbering a document whose
                    // clauses are cross-referenced by their printed label. A bullet defers to the
                    // label instead of competing with it.
                    string? sourceLabel = elemAttributes is not null
                        && elemAttributes.TryGetValue(InternalElement.ListItemSourceLabelAttribute, out var rawLabel)
                        && rawLabel.Length > 0
                        ? rawLabel
                        : null;
                    bool numbered = elemKind.Ordered && sourceLabel is null;
                    var itemList = new NodeList
                    {
                        ListType = numbered ? ListType.Ordered : ListType.Bullet,
                        BulletChar = (byte)'-',
                        Start = 1,
                        Tight = true,
                        Delimiter = ListDelimType.Period,
                    };
                    var item = new MdNode(NodeType.Item) { List = itemList };
                    var itemPara = new MdNode(NodeType.Paragraph);
                    if (sourceLabel is not null) itemPara.Append(MkText(sourceLabel + " "));
                    var (itemText, itemAnnotations) = StripRedundantListMarker(elemText, elemAnnotations);
                    BuildInlines(itemPara, itemText, itemAnnotations);
                    item.Append(itemPara);

                    MdNode listParent;
                    if (parent.Type == NodeType.List)
                    {
                        // A run opened by ListStart is already numbered; a labelled item joining it
                        // has to demote the whole run, or the writer resumes counting around it.
                        // NodeList is a struct, so the demoted copy has to be written back.
                        if (!numbered)
                        {
                            var openList = parent.List;
                            openList.ListType = ListType.Bullet;
                            openList.BulletChar = (byte)'-';
                            parent.List = openList;
                        }
                        listParent = parent;
                    }
                    else
                    {
                        var implicitList = new MdNode(NodeType.List) { List = itemList };
                        parent.Append(implicitList);
                        listParent = implicitList;
                    }
                    listParent.Append(item);
                    break;
                }
                case ElementKindTag.Code:
                {
                    string lang = elemAttributes is not null && elemAttributes.TryGetValue("language", out var l) ? l : "";
                    var codeBlock = new MdNode(NodeType.CodeBlock)
                    {
                        CodeBlock = new NodeCodeBlock
                        {
                            Fenced = true, FenceChar = (byte)'`', FenceLength = 3, FenceOffset = 0,
                            Info = lang, Literal = elemText, Closed = true,
                        },
                    };
                    parent.Append(codeBlock);
                    break;
                }
                case ElementKindTag.Formula:
                {
                    var math = new MdNode(NodeType.Math)
                    {
                        Math = new NodeMath { DollarMath = true, DisplayMath = true, Literal = elemText },
                    };
                    var para = new MdNode(NodeType.Paragraph);
                    para.Append(math);
                    parent.Append(para);
                    break;
                }
                case ElementKindTag.Table:
                {
                    int ti = (int)elemKind.TableIndex;
                    if (ti >= 0 && ti < doc.Tables.Count)
                    {
                        var table = doc.Tables[ti];
                        // Most formats mark their header row by writing one. A CSV, an org table or
                        // a typst table has a first row that may be data, and the extractor says
                        // which by whether it recorded `Columns`. Promoting that row anyway labels
                        // the first record as the column names.
                        bool hasHeader = doc.SourceFormat is not ("csv" or "orgmode" or "typst")
                            || table.Columns is not null;
                        // A grid whose every row is empty has no columns to align, and a table node
                        // with zero columns renders as a bare row of pipes. Fall through to the
                        // pre-rendered markdown, if the extractor left any.
                        var tableNode = BuildTable(table.Cells, hasHeader);
                        if (tableNode is not null)
                        {
                            parent.Append(tableNode);
                        }
                        else if (table.Markdown.Trim().Length > 0)
                        {
                            var para = new MdNode(NodeType.Paragraph);
                            para.Append(MkText(table.Markdown));
                            parent.Append(para);
                        }
                    }
                    break;
                }
                case ElementKindTag.Image:
                {
                    int ii = (int)elemKind.ImageIndex;
                    ExtractedImage? image = ii >= 0 && ii < doc.Images.Count ? doc.Images[ii] : null;
                    string desc = image?.Description ?? "";
                    string url;
                    if (image is null)
                    {
                        if (desc.Length == 0) continue;
                        url = "";
                    }
                    else if (image.Data.Length > 0) url = $"image_{elemKind.ImageIndex}.{image.Format}";
                    else if (image.SourcePath is not null) url = image.SourcePath;
                    else url = $"image_{elemKind.ImageIndex}.bin";

                    var para = new MdNode(NodeType.Paragraph);
                    var imgNode = new MdNode(NodeType.Image) { Link = new NodeLink { Url = url, Title = "" } };
                    imgNode.Append(MkText(desc));
                    para.Append(imgNode);
                    parent.Append(para);
                    break;
                }
                case ElementKindTag.FootnoteRef:
                {
                    uint? n = origIdx is int oi ? footnotes.RefNumber((uint)oi) : null;
                    if (n is uint num)
                    {
                        var fnref = new MdNode(NodeType.FootnoteReference)
                        {
                            FootnoteReference = new NodeFootnoteReference
                            {
                                Name = num.ToString(), RefNum = (int)num, Ix = (int)num,
                            },
                        };
                        MdNode inlineParent;
                        var last = parent.LastChild;
                        if (last is { Type: NodeType.Paragraph }) inlineParent = last;
                        else
                        {
                            inlineParent = new MdNode(NodeType.Paragraph);
                            parent.Append(inlineParent);
                        }
                        inlineParent.Append(fnref);
                    }
                    break;
                }
                case ElementKindTag.FootnoteDefinition:
                case ElementKindTag.Citation:
                case ElementKindTag.CommentDefinition:
                    break; // rendered at the end
                case ElementKindTag.CommentRef:
                    break;
                case ElementKindTag.PageBreak:
                    break;
                case ElementKindTag.Slide:
                {
                    parent.Append(new MdNode(NodeType.ThematicBreak));
                    if (elemText.Length > 0)
                    {
                        var heading = new MdNode(NodeType.Heading) { Heading = new NodeHeading { Level = 2 } };
                        BuildInlines(heading, elemText, elemAnnotations);
                        parent.Append(heading);
                    }
                    break;
                }
                case ElementKindTag.DefinitionTerm:
                {
                    var dt = new MdNode(NodeType.Paragraph);
                    BuildInlines(dt, elemText, elemAnnotations);
                    parent.Append(dt);
                    break;
                }
                case ElementKindTag.DefinitionDescription:
                {
                    var dd = new MdNode(NodeType.Paragraph);
                    BuildInlines(dd, ": " + elemText, Array.Empty<TextAnnotation>());
                    parent.Append(dd);
                    break;
                }
                case ElementKindTag.Admonition:
                {
                    string kindStr = elemAttributes is not null && elemAttributes.TryGetValue("kind", out var k) ? k : "note";
                    string? title = elemAttributes is not null && elemAttributes.TryGetValue("title", out var t) ? t : null;
                    AlertType? alertType = kindStr.ToLowerInvariant() switch
                    {
                        "note" => AlertType.Note,
                        "tip" or "hint" => AlertType.Tip,
                        "important" => AlertType.Important,
                        "warning" or "warn" => AlertType.Warning,
                        "caution" or "danger" or "error" => AlertType.Caution,
                        _ => null,
                    };
                    if (alertType is AlertType at)
                    {
                        var alert = new MdNode(NodeType.Alert)
                        {
                            Alert = new NodeAlert { AlertType = at, Title = title },
                        };
                        if (elemText.Length > 0)
                        {
                            var para = new MdNode(NodeType.Paragraph);
                            BuildInlines(para, elemText, elemAnnotations);
                            alert.Append(para);
                        }
                        parent.Append(alert);
                    }
                    else
                    {
                        var bq = new MdNode(NodeType.BlockQuote);
                        string titleDisplay = title ?? kindStr;
                        var titlePara = new MdNode(NodeType.Paragraph);
                        var strong = new MdNode(NodeType.Strong);
                        strong.Append(MkText(titleDisplay));
                        titlePara.Append(strong);
                        bq.Append(titlePara);
                        if (elemText.Length > 0)
                        {
                            var bodyPara = new MdNode(NodeType.Paragraph);
                            BuildInlines(bodyPara, elemText, elemAnnotations);
                            bq.Append(bodyPara);
                        }
                        parent.Append(bq);
                    }
                    break;
                }
                case ElementKindTag.RawBlock:
                {
                    parent.Append(new MdNode(NodeType.Raw) { Literal = elemText });
                    break;
                }
                case ElementKindTag.MetadataBlock:
                {
                    var entries = RenderCommon.ParseMetadataEntries(elemText);
                    if (entries.Count > 0)
                    {
                        foreach (var (key, value) in entries)
                        {
                            var para = new MdNode(NodeType.Paragraph);
                            var strong = new MdNode(NodeType.Strong);
                            strong.Append(MkText(key));
                            para.Append(strong);
                            para.Append(MkText(": " + value));
                            parent.Append(para);
                        }
                    }
                    else if (elemText.Length > 0)
                    {
                        var para = new MdNode(NodeType.Paragraph);
                        para.Append(MkText(elemText));
                        parent.Append(para);
                    }
                    break;
                }
                case ElementKindTag.OcrText:
                {
                    if (elemText.Length > 0)
                    {
                        // Recognised text the OCR pass marked as markdown goes through verbatim:
                        // as a paragraph's words it would be escaped, turning a recognised heading
                        // into `\## Heading` and a recognised table into its own source.
                        if (elemAttributes is { } attributes
                            && attributes.ContainsKey(OcrProcessor.MarkdownAttribute))
                        {
                            parent.Append(new MdNode(NodeType.Raw) { Literal = elemText });
                            break;
                        }

                        var para = new MdNode(NodeType.Paragraph);
                        BuildInlines(para, elemText, elemAnnotations);
                        parent.Append(para);
                    }
                    break;
                }
                case ElementKindTag.ListStart:
                {
                    state.PushContainer(NestingKind.ListKind(elemKind.Ordered, 0), elemDepth);
                    var listMeta = new NodeList
                    {
                        ListType = elemKind.Ordered ? ListType.Ordered : ListType.Bullet,
                        BulletChar = (byte)'-', Start = 1, Tight = true, Delimiter = ListDelimType.Period,
                    };
                    var listNode = new MdNode(NodeType.List) { List = listMeta };

                    MdNode target;
                    if (parent.Type == NodeType.List)
                    {
                        MdNode? lastItem = null;
                        foreach (var c in parent.Children())
                            if (c.Type is NodeType.Item or NodeType.TaskItem) lastItem = c;
                        if (lastItem is not null) target = lastItem;
                        else
                        {
                            var item = new MdNode(NodeType.Item) { List = listMeta };
                            parent.Append(item);
                            target = item;
                        }
                    }
                    else target = parent;
                    target.Append(listNode);
                    containerStack.Add(new ContainerEntry { Node = listNode, Kind = ContainerKind.List });
                    break;
                }
                case ElementKindTag.QuoteStart:
                {
                    state.PushContainer(NestingKind.BlockQuote, elemDepth);
                    var bq = new MdNode(NodeType.BlockQuote);
                    parent.Append(bq);
                    containerStack.Add(new ContainerEntry { Node = bq, Kind = ContainerKind.BlockQuote });
                    break;
                }
                case ElementKindTag.GroupStart:
                {
                    state.PushContainer(NestingKind.Group, elemDepth);
                    containerStack.Add(new ContainerEntry { Node = parent, Kind = ContainerKind.Group });
                    break;
                }
                case ElementKindTag.ListEnd:
                case ElementKindTag.QuoteEnd:
                case ElementKindTag.GroupEnd:
                    break; // handled above
            }
        }

        // Footnote definitions.
        foreach (var entry in footnotes.Definitions)
        {
            var fndef = new MdNode(NodeType.FootnoteDefinition)
            {
                FootnoteDefinition = new NodeFootnoteDefinition { Name = entry.Number.ToString(), TotalReferences = 1 },
            };
            var para = new MdNode(NodeType.Paragraph);
            para.Append(MkText(entry.Text));
            fndef.Append(para);
            root.Append(fndef);
        }

        // Citations (as footnote definitions).
        foreach (var elem in doc.Elements)
        {
            if (elem.Kind.Tag == ElementKindTag.Citation)
            {
                string key = elem.Anchor ?? "?";
                var fndef = new MdNode(NodeType.FootnoteDefinition)
                {
                    FootnoteDefinition = new NodeFootnoteDefinition { Name = key, TotalReferences = 1 },
                };
                var para = new MdNode(NodeType.Paragraph);
                para.Append(MkText(elem.Text));
                fndef.Append(para);
                root.Append(fndef);
            }
        }

        // Comment definitions are not tracked by the footnote collector, so they are surfaced
        // the same way citations are above — keyed by anchor — rather than dropped.
        foreach (var elem in doc.Elements)
        {
            if (elem.Kind.Tag == ElementKindTag.CommentDefinition)
            {
                string key = elem.Anchor ?? "?";
                var fndef = new MdNode(NodeType.FootnoteDefinition)
                {
                    FootnoteDefinition = new NodeFootnoteDefinition { Name = key, TotalReferences = 1 },
                };
                var para = new MdNode(NodeType.Paragraph);
                para.Append(MkText(elem.Text));
                fndef.Append(para);
                root.Append(fndef);
            }
        }

        return root;
    }

    private static void PopContainer(List<ContainerEntry> stack, ContainerKind target)
    {
        for (int i = stack.Count - 1; i >= 0; i--)
        {
            if (stack[i].Kind == target)
            {
                stack.RemoveAt(i);
                return;
            }
        }
    }

    // ---- node constructors ----------------------------------------------

    private static MdNode MkText(string text) =>
        new(NodeType.Text) { Literal = RenderCommon.NormalizeInlineText(text) };

    /// <summary>
    /// Build a pipe table. When <paramref name="hasHeader"/> is false an empty header row is
    /// synthesized ahead of the data, since GFM has no headerless table and promoting the first
    /// data row would relabel it as the column names.
    /// </summary>
    /// <summary>
    /// Build a table subtree from a 2-D cell grid, or <c>null</c> when the grid has no columns
    /// for the caller to fall back on its pre-rendered markdown.
    /// </summary>

    /// <summary>Lower bound comrak accepts for a heading level.</summary>
    private const byte MinHeadingLevel = 1;

    /// <summary>Upper bound comrak accepts for a heading level.</summary>
    private const byte MaxHeadingLevel = 6;

    /// <summary>Bullet glyphs a list item's own text may redundantly carry.</summary>
    private static readonly char[] BulletMarkerChars = { '-', '*', '\u2022', '\u2023', '\u25E6' };

    /// <summary>
    /// Length, in UTF-16 units, of a list marker at the head of <paramref name="text"/> that the
    /// enclosing list will render for itself — or 0 if the text carries none.
    /// </summary>
    /// <remarks>
    /// Structure detection reads a numbered marker as part of the line's own text, so an item's
    /// text can still start with "1. " even though the element is correctly flagged as an ordered
    /// list item. Left in place, the renderer emits its own "1. " marker followed by the untouched
    /// label — and the CommonMark writer then backslash-escapes that second period, to keep it from
    /// being misread as a second list start on re-parse, producing "1. 1\. Land Use…".
    /// </remarks>
    private static int RedundantListMarkerLength(string text)
    {
        if (text.Length == 0) return 0;

        if (Array.IndexOf(BulletMarkerChars, text[0]) >= 0)
        {
            int afterBullet = 1;
            int bulletSpaces = 0;
            while (afterBullet + bulletSpaces < text.Length && text[afterBullet + bulletSpaces] == ' ') bulletSpaces++;
            return bulletSpaces > 0 ? afterBullet + bulletSpaces : 0;
        }

        int digits = 0;
        while (digits < text.Length && text[digits] is >= '0' and <= '9') digits++;
        if (digits == 0 || digits >= text.Length) return 0;

        if (text[digits] is not ('.' or ')')) return 0;

        int afterDelimiter = digits + 1;
        int spaces = 0;
        while (afterDelimiter + spaces < text.Length && text[afterDelimiter + spaces] == ' ') spaces++;
        return spaces > 0 ? afterDelimiter + spaces : 0;
    }

    /// <summary>
    /// <paramref name="text"/> without the marker the enclosing list renders anyway, with the
    /// annotation offsets shifted to match. See <see cref="RedundantListMarkerLength"/>.
    /// </summary>
    private static (string Text, IReadOnlyList<TextAnnotation> Annotations) StripRedundantListMarker(
        string text, IReadOnlyList<TextAnnotation> annotations)
    {
        int markerLen = RedundantListMarkerLength(text);
        if (markerLen == 0) return (text, annotations);

        uint offset = (uint)Encoding.UTF8.GetByteCount(text.AsSpan(0, markerLen));
        var adjusted = new List<TextAnnotation>(annotations.Count);
        foreach (var ann in annotations)
        {
            if (ann.End <= offset) continue;
            adjusted.Add(new TextAnnotation
            {
                Start = ann.Start > offset ? ann.Start - offset : 0,
                End = ann.End - offset,
                Kind = ann.Kind,
            });
        }
        return (text[markerLen..], adjusted);
    }

    private static MdNode? BuildTable(IReadOnlyList<List<string>> cells, bool hasHeader = true)
    {
        int numCols = cells.Count > 0 ? cells.Max(r => r.Count) : 0;
        if (numCols == 0) return null;
        int nonEmpty = cells.Sum(r => r.Count(c => c.Length > 0));
        int syntheticHeaderRows = hasHeader ? 0 : 1;

        var tableNode = new MdNode(NodeType.Table)
        {
            Table = new NodeTable
            {
                Alignments = Enumerable.Repeat(TableAlignment.None, numCols).ToList(),
                NumColumns = numCols,
                NumRows = cells.Count + syntheticHeaderRows,
                NumNonemptyCells = nonEmpty,
            },
        };

        if (!hasHeader)
        {
            var emptyHeader = new MdNode(NodeType.TableRow) { TableRowHeader = true };
            for (int col = 0; col < numCols; col++) emptyHeader.Append(new MdNode(NodeType.TableCell));
            tableNode.Append(emptyHeader);
        }

        for (int rowIdx = 0; rowIdx < cells.Count; rowIdx++)
        {
            var row = cells[rowIdx];
            var rowNode = new MdNode(NodeType.TableRow) { TableRowHeader = hasHeader && rowIdx == 0 };
            for (int col = 0; col < numCols; col++)
            {
                var cellNode = new MdNode(NodeType.TableCell);
                string content = col < row.Count ? row[col] : "";
                if (content.Length > 0) cellNode.Append(MkText(content));
                rowNode.Append(cellNode);
            }
            tableNode.Append(rowNode);
        }
        return tableNode;
    }

    // ---- inline annotation building -------------------------------------

    private static void BuildInlines(MdNode parent, string text, IReadOnlyList<TextAnnotation> annotations)
    {
        if (annotations.Count == 0)
        {
            if (text.Length > 0) parent.Append(MkText(text));
            return;
        }

        byte[] bytes = Encoding.UTF8.GetBytes(text);
        uint len = (uint)bytes.Length;

        var sorted = annotations
            .Select((a, i) => (a, i))
            .OrderBy(t => t.a.Start)
            .ThenByDescending(t => t.a.End >= t.a.Start ? t.a.End - t.a.Start : 0u)
            .ThenBy(t => t.a.Kind.Which == AnnotationKind.Tag.Link ? 0 : 1)
            .Select(t => t.a)
            .ToList();

        uint pos = 0;
        int idx = 0;
        while (idx < sorted.Count)
        {
            var ann = sorted[idx];
            uint start = (uint)CeilCharBoundary(bytes, (int)Math.Min(ann.Start, len));
            uint end = (uint)FloorCharBoundary(bytes, (int)Math.Min(ann.End, len));

            if (start < pos) { idx++; continue; }
            if (start >= end) { idx++; continue; }

            if (start > pos)
            {
                string gap = Encoding.UTF8.GetString(bytes, (int)pos, (int)(start - pos));
                if (gap.Length > 0) parent.Append(MkText(gap));
            }

            string span = Encoding.UTF8.GetString(bytes, (int)start, (int)(end - start));

            var inner = new List<TextAnnotation>();
            for (int j = idx + 1; j < sorted.Count; j++)
            {
                var ia = sorted[j];
                uint iStart = (uint)CeilCharBoundary(bytes, (int)Math.Min(ia.Start, len));
                uint iEnd = (uint)FloorCharBoundary(bytes, (int)Math.Min(ia.End, len));
                if (iStart >= start && iEnd <= end && iStart < iEnd)
                    inner.Add(new TextAnnotation { Start = iStart - start, End = iEnd - start, Kind = ia.Kind });
            }

            AppendAnnotatedSpan(parent, span, ann.Kind, inner);
            pos = end;

            idx++;
            while (idx < sorted.Count)
            {
                uint ns = (uint)CeilCharBoundary(bytes, (int)Math.Min(sorted[idx].Start, len));
                if (ns < end) idx++;
                else break;
            }
        }

        if (pos < bytes.Length)
        {
            string tail = Encoding.UTF8.GetString(bytes, (int)pos, (int)(len - pos));
            if (tail.Length > 0) parent.Append(MkText(tail));
        }
    }

    private static void AppendAnnotatedSpan(MdNode parent, string span, AnnotationKind kind, List<TextAnnotation> inner)
    {
        string leadingWs = "", trimmed = span, trailingWs = "";
        bool isEmphKind = kind.Which is AnnotationKind.Tag.Bold or AnnotationKind.Tag.Italic or AnnotationKind.Tag.Strikethrough;
        if (isEmphKind)
        {
            trimmed = span.Trim();
            if (trimmed.Length == 0)
            {
                if (span.Length > 0) parent.Append(MkText(span));
                return;
            }
            leadingWs = span.Substring(0, span.Length - span.TrimStart().Length);
            trailingWs = span.Substring(span.TrimEnd().Length);
        }

        uint leadingLen = (uint)Encoding.UTF8.GetByteCount(leadingWs);
        uint trimmedLen = (uint)Encoding.UTF8.GetByteCount(trimmed);
        List<TextAnnotation> innerForNode;
        if (leadingLen == 0) innerForNode = inner;
        else
        {
            innerForNode = new List<TextAnnotation>();
            foreach (var ia in inner)
            {
                uint ist = ia.Start >= leadingLen ? ia.Start - leadingLen : 0;
                uint ie = Math.Min(ia.End >= leadingLen ? ia.End - leadingLen : 0, trimmedLen);
                if (ist < ie) innerForNode.Add(new TextAnnotation { Start = ist, End = ie, Kind = ia.Kind });
            }
        }

        if (leadingWs.Length > 0) parent.Append(MkText(leadingWs));

        switch (kind.Which)
        {
            case AnnotationKind.Tag.Bold:
            {
                var strong = new MdNode(NodeType.Strong);
                BuildInlines(strong, trimmed, innerForNode);
                parent.Append(strong);
                break;
            }
            case AnnotationKind.Tag.Italic:
            {
                var emph = new MdNode(NodeType.Emph);
                BuildInlines(emph, trimmed, innerForNode);
                parent.Append(emph);
                break;
            }
            case AnnotationKind.Tag.Code:
            {
                if (trimmed.Length > 0)
                {
                    var code = new MdNode(NodeType.Code)
                    {
                        Code = new NodeCode { NumBackticks = 1, Literal = RenderCommon.NormalizeInlineText(trimmed) },
                    };
                    parent.Append(code);
                }
                break;
            }
            case AnnotationKind.Tag.Strikethrough:
            {
                var strike = new MdNode(NodeType.Strikethrough);
                BuildInlines(strike, trimmed, innerForNode);
                parent.Append(strike);
                break;
            }
            case AnnotationKind.Tag.Underline:
            {
                var u = new MdNode(NodeType.Underline);
                BuildInlines(u, trimmed, innerForNode);
                parent.Append(u);
                break;
            }
            case AnnotationKind.Tag.Subscript:
            {
                var sub = new MdNode(NodeType.Subscript);
                BuildInlines(sub, trimmed, innerForNode);
                parent.Append(sub);
                break;
            }
            case AnnotationKind.Tag.Superscript:
            {
                var sup = new MdNode(NodeType.Superscript);
                BuildInlines(sup, trimmed, innerForNode);
                parent.Append(sup);
                break;
            }
            case AnnotationKind.Tag.Highlight:
            {
                var hl = new MdNode(NodeType.Highlight);
                BuildInlines(hl, trimmed, innerForNode);
                parent.Append(hl);
                break;
            }
            case AnnotationKind.Tag.Link:
            {
                var link = new MdNode(NodeType.Link)
                {
                    Link = new NodeLink { Url = kind.Url ?? "", Title = kind.Title ?? "" },
                };
                BuildInlines(link, trimmed, inner);
                parent.Append(link);
                break;
            }
            default: // Color, FontSize, Custom
                parent.Append(MkText(trimmed));
                break;
        }

        if (trailingWs.Length > 0) parent.Append(MkText(trailingWs));
    }

    private static int CeilCharBoundary(byte[] bytes, int index)
    {
        if (index >= bytes.Length) return bytes.Length;
        int i = index;
        while (i < bytes.Length && (bytes[i] & 0xC0) == 0x80) i++;
        return i;
    }

    private static int FloorCharBoundary(byte[] bytes, int index)
    {
        if (index >= bytes.Length) return bytes.Length;
        int i = index;
        while (i > 0 && (bytes[i] & 0xC0) == 0x80) i--;
        return i;
    }

    // ---- paragraph consolidation ----------------------------------------

    private sealed class ConsolidatedElement
    {
        public int OriginalIndex;
        public string? MergedText;
        public List<TextAnnotation>? MergedAnnotations;
    }

    private static List<ConsolidatedElement> ConsolidateParagraphs(List<InternalElement> elements)
    {
        var result = new List<ConsolidatedElement>(elements.Count);
        int i = 0;
        while (i < elements.Count)
        {
            Cancel.Every(i, 0x3FF);
            var elem = elements[i];
            AnnotationKind? uniform = null;
            if (elem.Kind.Tag == ElementKindTag.Paragraph && elem.Layer == ContentLayer.Body && elem.Text.Length > 0)
                uniform = UniformAnnotationKind(elem);

            if (uniform is not null)
            {
                string mergedText = elem.Text;
                int j = i + 1;
                while (j < elements.Count)
                {
                    if (EndsAtSentenceBoundary(mergedText)) break;
                    var next = elements[j];
                    if (next.Kind.Tag != ElementKindTag.Paragraph || next.Layer != ContentLayer.Body || next.Text.Length == 0)
                        break;
                    var nextKind = UniformAnnotationKind(next);
                    if (nextKind is not null && nextKind.Which == uniform.Which)
                    {
                        mergedText += " " + next.Text;
                        j++;
                        continue;
                    }
                    break;
                }

                if (j > i + 1)
                {
                    var ann = new TextAnnotation
                    {
                        Start = 0,
                        End = (uint)Encoding.UTF8.GetByteCount(mergedText),
                        Kind = uniform,
                    };
                    result.Add(new ConsolidatedElement
                    {
                        MergedText = mergedText,
                        MergedAnnotations = new List<TextAnnotation> { ann },
                    });
                    i = j;
                    continue;
                }
            }

            result.Add(new ConsolidatedElement { OriginalIndex = i });
            i++;
        }
        return result;
    }

    private static AnnotationKind? UniformAnnotationKind(InternalElement elem)
    {
        AnnotationKind? formatting = null;
        int textLen = Encoding.UTF8.GetByteCount(elem.Text);
        foreach (var ann in elem.Annotations)
        {
            switch (ann.Kind.Which)
            {
                case AnnotationKind.Tag.Bold:
                case AnnotationKind.Tag.Italic:
                case AnnotationKind.Tag.Strikethrough:
                    if (ann.Start == 0 && ann.End >= textLen)
                    {
                        if (formatting is not null) return null;
                        formatting = ann.Kind;
                    }
                    else return null;
                    break;
            }
        }
        return formatting;
    }

    private static bool EndsAtSentenceBoundary(string text)
    {
        string trimmed = text.TrimEnd();
        return trimmed.EndsWith('.') || trimmed.EndsWith('!') || trimmed.EndsWith('?') || trimmed.EndsWith(':');
    }
}
