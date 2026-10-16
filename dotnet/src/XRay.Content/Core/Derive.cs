using XRay.Content.Rendering;
using XRay.Content.Types;

namespace XRay.Content.Core;

/// <summary>
/// Derives the public <see cref="ExtractedDocument"/> from an <see cref="InternalDocument"/>.
/// Ports the NATIVE happy path of Rust `extraction/derive.rs`.
///
/// Deferred (documented in PORT_NOTES): OCR element building, chunking, embeddings, keyword
/// extraction, LLM usage, tree-sitter code intelligence, element-based output (`elements`),
/// and element-based output (`elements`). The Rust
/// pipeline's later `content = formatted_content` swap is folded in here so `Content` already
/// reflects the requested output format.
///
/// Per-page re-render in the requested markup is opt-in — see
/// <see cref="ExtractionConfig.RenderPagesInOutputFormat"/> — because the plain per-page content
/// is what upstream produces and what the goldens pin.
/// </summary>
public static class Derive
{
    public static ExtractedDocument DeriveExtractionResult(
        InternalDocument doc, bool includeDocumentStructure, OutputFormat outputFormat,
        HtmlOutputConfig? htmlOutput = null, bool renderPagesInOutputFormat = false)
    {
        ResolveRelationships(doc);

        string content = PlainRenderer.Render(doc);

        string mimeType = doc.MimeType != Mime.OctetStream
            ? doc.MimeType
            : SourceFormatToMimeType(doc.SourceFormat);

        string? formatted = RenderFormatted(doc, outputFormat, htmlOutput);

        List<PageContent>? pages = doc.PrebuiltPages ?? BuildPages(doc);

        // Not for a page the extractor prebuilt: that content is already rendered, and it carries
        // what the element stream does not — a sheet's name as its heading, say — so re-rendering
        // the page's elements would lose it.
        if (renderPagesInOutputFormat && pages is not null && formatted is not null && doc.PrebuiltPages is null)
            RenderPagesFormatted(doc, pages, outputFormat, htmlOutput);

        DocumentStructure? document = includeDocumentStructure ? DeriveDocumentStructure(doc) : null;

        List<ExtractedImage>? images = doc.Images.Count == 0 ? null : doc.Images;

        List<ExtractedUri>? uris = DedupUris(doc.Uris);

        ExtractionMethod? method = ParseExtractionMethod(doc.Metadata);

        var result = new ExtractedDocument
        {
            Content = formatted ?? content,
            MimeType = mimeType,
            Metadata = doc.Metadata,
            ExtractionMethod = method,
            Tables = doc.Tables,
            Images = images,
            Pages = pages,
            Document = document,
            ProcessingWarnings = doc.ProcessingWarnings,
            Children = doc.Children,
            Attachments = doc.Attachments,
            FormFields = doc.FormFields,
            Annotations = doc.Annotations,
            Uris = uris,
            FormattedContent = formatted,
            PlainContent = content,
        };
        return result;
    }

    /// <summary>
    /// Render each page's own elements in the requested output format, into
    /// <see cref="PageContent.FormattedContent"/>.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <see cref="PageContent.Content"/> is a concatenation of the page's element texts, which is
    /// what upstream produces — and it is not the document's markup: a heading loses its hashes
    /// and a table, whose element carries no text at all, disappears entirely. A caller that
    /// wants the document as markdown *and* wants to know which page each part came from could
    /// therefore have one or the other, and had to re-extract a page at a time to get both.
    /// </para>
    /// <para>
    /// Each page is rendered from a slice of the document carrying only that page's elements, so
    /// the renderer resolves tables and images through the same indices. A page whose elements are
    /// not marked with its number keeps the content it came with, and a document whose extractor
    /// supplied <see cref="InternalDocument.PrebuiltPages"/> is skipped entirely: those pages are
    /// already rendered, and they carry what the element stream does not.
    /// </para>
    /// </remarks>
    private static void RenderPagesFormatted(
        InternalDocument doc, List<PageContent> pages, OutputFormat outputFormat,
        HtmlOutputConfig? htmlOutput)
    {
        var byPage = new Dictionary<uint, List<InternalElement>>();
        foreach (var elem in doc.Elements)
        {
            if (elem.Page is not uint page) continue;
            if (!byPage.TryGetValue(page, out var list)) byPage[page] = list = new List<InternalElement>();
            list.Add(elem);
        }

        foreach (var page in pages)
        {
            if (!byPage.TryGetValue(page.PageNumber, out var elements)) continue;

            var slice = new InternalDocument(doc.SourceFormat)
            {
                MimeType = doc.MimeType,
                Metadata = doc.Metadata,
            };
            // By reference and in full, because an element addresses a table or an image by its
            // index in the whole document.
            slice.Tables.AddRange(doc.Tables);
            slice.Images.AddRange(doc.Images);
            slice.Elements.AddRange(elements);

            page.FormattedContent = RenderFormatted(slice, outputFormat, htmlOutput) ?? page.Content;
        }
    }

    private static string? RenderFormatted(
        InternalDocument doc, OutputFormat outputFormat, HtmlOutputConfig? htmlOutput)
    {
        switch (outputFormat.Which)
        {
            case OutputFormat.Kind.Plain:
            case OutputFormat.Kind.Structured:
                return null;
            case OutputFormat.Kind.Markdown:
                if (doc.PreRenderedContent is not null && doc.Metadata.OutputFormat == "markdown")
                    return doc.PreRenderedContent;
                return MarkdownRenderer.Render(doc);
            case OutputFormat.Kind.Djot:
                if (doc.PreRenderedContent is not null && doc.Metadata.OutputFormat == "djot")
                    return doc.PreRenderedContent;
                return DjotRenderer.Render(doc);
            case OutputFormat.Kind.Html:
                // A caller who supplied an html_output config asked for the styled renderer;
                // without one the markdown-based renderer stays, which is what the goldens
                // measure and what upstream does when the config is `None`.
                return htmlOutput is null
                    ? HtmlRenderer.Render(doc)
                    : new StyledHtmlRenderer(htmlOutput).Render(doc);
            case OutputFormat.Kind.Json:
                return JsonRenderer.Render(doc);
            case OutputFormat.Kind.DocTags:
                return DocTagsRenderer.Render(doc);
            default:
                return null; // Custom renderer registry — deferred.
        }
    }

    private static void ResolveRelationships(InternalDocument doc)
    {
        // Build anchor -> first element index (skipping FootnoteRef anchors).
        var anchorToIndex = new Dictionary<string, uint>();
        for (int i = 0; i < doc.Elements.Count; i++)
        {
            var elem = doc.Elements[i];
            if (elem.Kind.Tag == ElementKindTag.FootnoteRef) continue;
            if (elem.Anchor is not null && !anchorToIndex.ContainsKey(elem.Anchor))
                anchorToIndex[elem.Anchor] = (uint)i;
        }
        foreach (var rel in doc.Relationships)
        {
            if (rel.Target.Key is string key && anchorToIndex.TryGetValue(key, out var idx))
                rel.Target = RelationshipTarget.FromIndex(idx);
        }
    }

    private static ExtractionMethod? ParseExtractionMethod(Metadata metadata)
    {
        if (metadata.Additional.TryGetValue("extraction_method", out var v)
            && v.ValueKind == System.Text.Json.JsonValueKind.String)
        {
            return v.GetString() switch
            {
                "native" => ExtractionMethod.Native,
                "ocr" => ExtractionMethod.Ocr,
                "mixed" => ExtractionMethod.Mixed,
                _ => null,
            };
        }
        return null;
    }

    private static List<ExtractedUri>? DedupUris(List<ExtractedUri> uris)
    {
        if (uris.Count == 0) return null;
        var seen = new HashSet<(string, Types.UriKind)>();
        var result = new List<ExtractedUri>();
        foreach (var u in uris)
        {
            if (seen.Add((u.Url, u.Kind))) result.Add(u);
        }
        return result;
    }

    private static List<PageContent>? BuildPages(InternalDocument doc)
    {
        var pageMap = new SortedDictionary<uint, List<InternalElement>>();
        foreach (var elem in doc.Elements)
        {
            if (elem.Page is uint p)
            {
                if (!pageMap.TryGetValue(p, out var list))
                {
                    list = new List<InternalElement>();
                    pageMap[p] = list;
                }
                list.Add(elem);
            }
        }
        if (pageMap.Count == 0) return null;

        var pages = new List<PageContent>();
        foreach (var (pageNum, elems) in pageMap)
        {
            var content = new System.Text.StringBuilder();
            var tables = new List<Table>();
            var imageIndices = new List<uint>();
            foreach (var elem in elems)
            {
                if (elem.Kind.IsContainerStart || elem.Kind.IsContainerEnd) continue;
                if (elem.Kind.Tag == ElementKindTag.Table)
                {
                    int ti = (int)elem.Kind.TableIndex;
                    if (ti >= 0 && ti < doc.Tables.Count) tables.Add(doc.Tables[ti]);
                }
                else if (elem.Kind.Tag == ElementKindTag.Image)
                {
                    if (elem.Kind.ImageIndex < doc.Images.Count) imageIndices.Add(elem.Kind.ImageIndex);
                }
                if (elem.Text.Length > 0)
                {
                    if (content.Length > 0) content.Append("\n\n");
                    content.Append(elem.Text);
                }
            }
            pages.Add(new PageContent
            {
                PageNumber = pageNum,
                Content = content.ToString(),
                Tables = tables,
                ImageIndices = imageIndices,
            });
        }
        return pages;
    }

    // ------------------------------------------------------------------
    // Document structure derivation (stack-based flat → tree builder)
    // ------------------------------------------------------------------

    private static DocumentStructure DeriveDocumentStructure(InternalDocument doc)
    {
        var ds = new DocumentStructure { SourceFormat = doc.SourceFormat };
        var nodes = ds.Nodes;
        var stack = new List<(ushort Depth, int NodeIndex)>();
        var elemToNode = new int?[doc.Elements.Count];
        var consumed = new bool[doc.Elements.Count];

        // Precompute definition term/description pairs.
        var defPairs = new Dictionary<int, int>();
        for (int i = 0; i + 1 < doc.Elements.Count; i++)
        {
            if (doc.Elements[i].Kind.Tag == ElementKindTag.DefinitionTerm
                && doc.Elements[i + 1].Kind.Tag == ElementKindTag.DefinitionDescription)
            {
                defPairs[i] = i + 1;
                consumed[i + 1] = true;
            }
        }

        void PopToDepth(ushort target)
        {
            while (stack.Count > 0 && stack[^1].Depth >= target)
                stack.RemoveAt(stack.Count - 1);
        }

        int PushNode(NodeContent content, InternalElement elem, List<TextAnnotation> annotations)
        {
            var node = new DocumentNode
            {
                Content = content,
                Parent = stack.Count > 0 ? (uint?)stack[^1].NodeIndex : null,
                ContentLayer = elem.Layer,
                Page = elem.Page,
                Bbox = elem.Bbox,
                Annotations = annotations,
                Attributes = elem.Attributes is null ? null : new Dictionary<string, string>(elem.Attributes),
            };
            int idx = nodes.Count;
            nodes.Add(node);
            if (stack.Count > 0)
                nodes[stack[^1].NodeIndex].Children.Add((uint)idx);
            return idx;
        }

        for (int i = 0; i < doc.Elements.Count; i++)
        {
            if (consumed[i]) continue;
            var elem = doc.Elements[i];
            var tag = elem.Kind.Tag;

            if (tag is ElementKindTag.ListEnd or ElementKindTag.QuoteEnd or ElementKindTag.GroupEnd)
            {
                if (stack.Count > 0)
                {
                    var topContent = nodes[stack[^1].NodeIndex].Content.Which;
                    bool matches = (tag == ElementKindTag.ListEnd && topContent == NodeContent.Tag.List)
                                   || (tag == ElementKindTag.QuoteEnd && topContent == NodeContent.Tag.Quote)
                                   || (tag == ElementKindTag.GroupEnd && topContent == NodeContent.Tag.Group);
                    if (matches) stack.RemoveAt(stack.Count - 1);
                }
                continue;
            }

            if (tag == ElementKindTag.FootnoteRef) continue;

            if (elem.Kind.IsContainerStart)
            {
                PopToDepth(elem.Depth);
                NodeContent content = tag switch
                {
                    ElementKindTag.ListStart => NodeContent.List(elem.Kind.Ordered),
                    ElementKindTag.QuoteStart => NodeContent.Quote(),
                    _ => new NodeContent
                    {
                        Which = NodeContent.Tag.Group,
                        Label = elem.Attributes is not null && elem.Attributes.TryGetValue("label", out var l) ? l : null,
                    },
                };
                int node = PushNode(content, elem, new());
                elemToNode[i] = node;
                stack.Add((elem.Depth, node));
                continue;
            }

            if (tag == ElementKindTag.Heading)
            {
                PopToDepth(elem.Depth);
                string headingText = elem.Text;
                var group = new NodeContent
                {
                    Which = NodeContent.Tag.Group,
                    HeadingLevel = elem.Kind.Level,
                    HeadingText = headingText,
                };
                int groupIdx = PushNode(group, elem, new());
                // Heading child inside the group.
                var headingChild = new DocumentNode
                {
                    Content = NodeContent.Heading(elem.Kind.Level, headingText),
                    Parent = (uint?)groupIdx,
                    ContentLayer = elem.Layer,
                    Page = elem.Page,
                    Bbox = elem.Bbox,
                    Annotations = elem.Annotations,
                };
                int headingIdx = nodes.Count;
                nodes.Add(headingChild);
                nodes[groupIdx].Children.Add((uint)headingIdx);
                elemToNode[i] = groupIdx;
                stack.Add((elem.Depth, groupIdx));
                continue;
            }

            if (defPairs.TryGetValue(i, out int descIdx))
            {
                PopToDepth(elem.Depth);
                EnsureDefinitionList(nodes, stack, elem, PushNode);
                var node = PushNode(NodeContent.DefinitionItem(elem.Text, doc.Elements[descIdx].Text), elem, new());
                elemToNode[i] = node;
                elemToNode[descIdx] = node;
                continue;
            }

            if (tag is ElementKindTag.DefinitionTerm or ElementKindTag.DefinitionDescription)
            {
                PopToDepth(elem.Depth);
                EnsureDefinitionList(nodes, stack, elem, PushNode);
                var content = ElementToNodeContent(elem, doc);
                var node = PushNode(content, elem, elem.Annotations);
                elemToNode[i] = node;
                continue;
            }

            // Close an open DefinitionList before a non-definition element.
            if (stack.Count > 0 && nodes[stack[^1].NodeIndex].Content.Which == NodeContent.Tag.DefinitionList)
                stack.RemoveAt(stack.Count - 1);

            PopToDepth(elem.Depth);
            var nc = ElementToNodeContent(elem, doc);
            int nodeIdx = PushNode(nc, elem, elem.Annotations);
            elemToNode[i] = nodeIdx;
        }

        // Relationships.
        foreach (var rel in doc.Relationships)
        {
            if (rel.Target.Index is not uint tgtElem) continue;
            int? sourceNode = SourceNodeFor(elemToNode, (int)rel.Source);
            int? targetNode = tgtElem < elemToNode.Length ? elemToNode[(int)tgtElem] : null;
            if (sourceNode is int s && targetNode is int t)
                ds.Relationships.Add(new DocumentRelationship { Source = (uint)s, Target = (uint)t, Kind = rel.Kind });
        }

        ds.FinalizeNodeTypes();
        return ds;
    }

    private static int? SourceNodeFor(int?[] elemToNode, int source)
    {
        for (int i = Math.Min(source, elemToNode.Length - 1); i >= 0; i--)
            if (elemToNode[i] is int n) return n;
        return null;
    }

    private static void EnsureDefinitionList(List<DocumentNode> nodes, List<(ushort Depth, int NodeIndex)> stack,
        InternalElement elem, Func<NodeContent, InternalElement, List<TextAnnotation>, int> pushNode)
    {
        if (stack.Count > 0 && nodes[stack[^1].NodeIndex].Content.Which == NodeContent.Tag.DefinitionList)
            return;
        int idx = pushNode(NodeContent.DefinitionList(), elem, new());
        stack.Add((elem.Depth, idx));
    }

    private static NodeContent ElementToNodeContent(InternalElement elem, InternalDocument doc)
    {
        switch (elem.Kind.Tag)
        {
            case ElementKindTag.Title: return NodeContent.Title(elem.Text);
            case ElementKindTag.Paragraph: return NodeContent.Paragraph(elem.Text);
            case ElementKindTag.ListItem: return NodeContent.ListItem(elem.Text);
            case ElementKindTag.Formula: return NodeContent.Formula(elem.Text);
            case ElementKindTag.FootnoteDefinition: return NodeContent.Footnote(elem.Text);
            case ElementKindTag.Code:
                return NodeContent.Code(elem.Text, RenderCommon.GetLanguage(elem));
            case ElementKindTag.Citation:
                return NodeContent.Citation(elem.Anchor ?? "", elem.Text);
            case ElementKindTag.Table:
                {
                    int ti = (int)elem.Kind.TableIndex;
                    var grid = ti >= 0 && ti < doc.Tables.Count ? TableToGrid(doc.Tables[ti]) : new TableGrid();
                    return NodeContent.Table(grid);
                }
            case ElementKindTag.Image:
                {
                    int ii = (int)elem.Kind.ImageIndex;
                    string? description = ii >= 0 && ii < doc.Images.Count ? doc.Images[ii].Description : null;
                    string? src = elem.Attributes is not null && elem.Attributes.TryGetValue("src", out var s) ? s : null;
                    return new NodeContent
                    {
                        Which = NodeContent.Tag.Image,
                        Description = description,
                        ImageIndex = elem.Kind.ImageIndex,
                        Src = src,
                    };
                }
            case ElementKindTag.PageBreak: return NodeContent.PageBreak();
            case ElementKindTag.Slide:
                return new NodeContent
                {
                    Which = NodeContent.Tag.Slide,
                    Number = elem.Kind.Number,
                    SlideTitle = elem.Text.Length > 0 ? elem.Text : null,
                };
            case ElementKindTag.DefinitionTerm:
                return NodeContent.DefinitionItem(elem.Text, "");
            case ElementKindTag.DefinitionDescription:
                return NodeContent.DefinitionItem("", elem.Text);
            case ElementKindTag.Admonition:
                return new NodeContent
                {
                    Which = NodeContent.Tag.Admonition,
                    Kind = RenderCommon.GetAdmonitionKind(elem),
                    SlideTitle = RenderCommon.GetAdmonitionTitle(elem),
                };
            case ElementKindTag.RawBlock:
                return new NodeContent
                {
                    Which = NodeContent.Tag.RawBlock,
                    Format = elem.Attributes is not null && elem.Attributes.TryGetValue("format", out var f) ? f : "",
                    RawContent = elem.Text,
                };
            case ElementKindTag.MetadataBlock:
                {
                    var entries = RenderCommon.ParseMetadataEntries(elem.Text)
                        .Select(e => new[] { e.Key, e.Value }).ToList();
                    return new NodeContent { Which = NodeContent.Tag.MetadataBlock, Entries = entries };
                }
            case ElementKindTag.OcrText:
                return NodeContent.Paragraph(elem.Text);
            default:
                return NodeContent.Paragraph(elem.Text);
        }
    }

    private static TableGrid TableToGrid(Table table)
    {
        var grid = new TableGrid();
        int rows = table.Cells.Count;
        int cols = table.Cells.Count > 0 ? table.Cells.Max(r => r.Count) : 0;
        grid.Rows = (uint)rows;
        grid.Cols = (uint)cols;
        for (int r = 0; r < rows; r++)
        {
            var row = table.Cells[r];
            for (int c = 0; c < row.Count; c++)
            {
                grid.Cells.Add(new GridCell
                {
                    Content = row[c],
                    Row = (uint)r,
                    Col = (uint)c,
                    RowSpan = 1,
                    ColSpan = 1,
                    IsHeader = r == 0,
                });
            }
        }
        return grid;
    }

    private static string SourceFormatToMimeType(string sourceFormat) => sourceFormat switch
    {
        "pdf" => "application/pdf",
        "html" => "text/html",
        "markdown" => "text/markdown",
        "text" => "text/plain",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        _ => Mime.OctetStream,
    };
}
