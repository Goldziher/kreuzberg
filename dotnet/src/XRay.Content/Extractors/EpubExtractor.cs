// Ported from crates/xberg/src/extractors/epub/mod.rs
// Native EPUB extractor: ZIP container -> OPF (manifest/spine/Dublin Core) -> XHTML spine walk.
//
// Deferrals vs. Rust (see PORT report):
//  - image_kind::classify is skipped (ImageKind/KindConfidence left null).
//  - Markdown/Djot pre-rendering (config.output_format == Markdown|Djot) is not performed;
//    this port always emits the structural element stream (plain/json parity target).
//  - config.include_document_structure is not consulted (structure is derived downstream).

using System.IO.Compression;
using XRay.Content.Core;
using XRay.Content.Internal.Epub;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>EPUB (EPUB2/EPUB3) extractor. Ported from `extractors/epub/mod.rs`.</summary>
public sealed class EpubExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[]
    {
        "application/epub+zip",
        "application/x-epub+zip",
        "application/vnd.epub+zip",
    };

    public int Priority => 60;

    private static readonly string[] MarkupSwitchNamespaces =
        { EpubContent.XhtmlNamespace, EpubContent.MathmlNamespace };

    private static readonly string[] PlainSwitchNamespaces = { EpubContent.XhtmlNamespace };

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        byte[] bytes = content.ToArray();
        using var ms = new MemoryStream(bytes, writable: false);
        using var archive = new ZipArchive(ms, ZipArchiveMode.Read);

        string containerXml = EpubContainer.ReadFileFromZip(archive, "META-INF/container.xml");
        string opfPath = EpubContainer.ParseContainerXml(containerXml);

        int lastSlash = opfPath.LastIndexOf('/');
        string manifestDir = lastSlash >= 0 ? opfPath.Substring(0, lastSlash) : "";

        string opfXml = EpubContainer.ReadFileFromZip(archive, opfPath);
        var (package, processingWarnings) = EpubOpf.ParseOpf(opfXml, manifestDir);

        var additionalMetadata = EpubOpf.BuildAdditionalMetadata(package.Metadata);

        var epubFormatMetadata = new FormatMetadata
        {
            FormatType = "epub",
            Payload = new EpubMetadata
            {
                Coverage = package.Metadata.Coverage,
                DcFormat = package.Metadata.Format,
                Relation = package.Metadata.Relation,
                Source = package.Metadata.Source,
                DcType = package.Metadata.DcType,
                CoverImage = package.Metadata.CoverImageHref,
            },
        };

        var (spineDocuments, bodyWarnings) = EpubContent.ReadBodyDocuments(archive, package);
        processingWarnings.AddRange(bodyWarnings);

        // `epub:switch` branches are resolved per renderer: the markup renderers draw MathML,
        // the plain/structured path does not, so each selects a different branch (mod.rs, the
        // MARKUP_SWITCH_NAMESPACES / PLAIN_SWITCH_NAMESPACES pair).
        bool wantsMarkup = config.OutputFormat.Which is Core.OutputFormat.Kind.Markdown
            or Core.OutputFormat.Kind.Djot;
        string[] supportedNamespaces = wantsMarkup ? MarkupSwitchNamespaces : PlainSwitchNamespaces;
        foreach (var spineDoc in spineDocuments)
        {
            Cancel.ThrowIfRequested();
            spineDoc.Xhtml = EpubContent.ResolveEpubSwitchElements(spineDoc.Xhtml, supportedNamespaces);
        }

        string? coverImagePath = package.Metadata.CoverImageHref;
        var doc = BuildInternalDocument(archive, spineDocuments, coverImagePath, config.SecurityLimits)
                  ?? new InternalDocumentBuilder("epub").Build();

        doc.MimeType = mimeType;
        doc.ProcessingWarnings.AddRange(processingWarnings);

        doc.Metadata = new Metadata
        {
            Title = package.Metadata.Title,
            Authors = package.Metadata.Creators.Count > 0 ? new List<string>(package.Metadata.Creators) : null,
            Keywords = package.Metadata.Subjects.Count > 0 ? new List<string>(package.Metadata.Subjects) : null,
            Language = package.Metadata.Language,
            CreatedAt = package.Metadata.Date,
            Format = epubFormatMetadata,
            Additional = additionalMetadata,
        };

        // Pre-rendered markdown (epub/mod.rs "Accumulate pre-rendered markdown"):
        // every spine document is converted BEFORE the navigation-document skip, the
        // fragments joined with a blank line, line trailing-whitespace trimmed, and a
        // single trailing newline appended. Stored as pre_rendered_content so the
        // pipeline returns it verbatim instead of the comrak re-render.
        if (config.OutputFormat.Which == Core.OutputFormat.Kind.Markdown && spineDocuments.Count > 0)
        {
            var fragments = new List<string>(spineDocuments.Count);
            foreach (var spineDoc in spineDocuments)
                fragments.Add(Internal.Html.HtmlToMarkdown.Convert(spineDoc.Xhtml).TrimEnd('\n', '\r'));

            string combined = string.Join("\n\n", fragments);
            combined = string.Join("\n", combined.Split('\n').Select(l => l.TrimEnd()));
            string trimmed = combined.TrimEnd();
            if (trimmed.Length > 0)
            {
                doc.PreRenderedContent = trimmed + "\n";
                doc.Metadata.OutputFormat = "markdown";
            }
        }

        return doc;
    }

    /// <summary>Build the flat <see cref="InternalDocument"/> from the spine. Mirrors `build_internal_document`.</summary>
    private static InternalDocument BuildInternalDocument(
        ZipArchive archive, List<EpubSpineDocument> spineDocuments, string? coverImagePath,
        SecurityLimits? limits)
    {
        // An EPUB stops at its limit rather than failing on it: a book that runs the budget out
        // mid-spine keeps the chapters already read. That is upstream's choice here specifically,
        // which is why these calls report instead of throwing.
        var budget = new SecurityBudget(limits ?? new SecurityLimits());
        var builder = new InternalDocumentBuilder("epub");

        // Emit the cover image as the first element, if present.
        if (coverImagePath is not null)
        {
            byte[] buf = EpubContainer.ReadBytesFromZip(archive, coverImagePath);
            if (buf.Length > 0)
            {
                string fmt = FormatFromPath(coverImagePath);
                var image = new ExtractedImage
                {
                    Data = buf,
                    Format = fmt,
                    ImageIndex = 0,
                    PageNumber = 0,
                    IsMask = false,
                    Description = "Cover",
                };
                builder.PushImage("Cover", image, null, null);
            }
        }

        for (int index = 0; index < spineDocuments.Count; index++)
        {
            if (!budget.TryStep()) break;

            var spineDoc = spineDocuments[index];
            string filePath = spineDoc.FilePath;
            string sanitized = spineDoc.Xhtml;

            // No navigation check here any more: the spine walk already dropped the items the
            // package itself names as navigation, and re-running the content heuristic on every
            // document dropped bibliographies, endnotes and licence pages that markdown output
            // kept — so the two formats disagreed.
            budget.TryAccountText(System.Text.Encoding.UTF8.GetByteCount(sanitized));

            // Skip empty chapters, unless the page carries image markup: a cover, a plate or a
            // fixed-layout page has no text of its own and still belongs in the document.
            if (EpubContent.ExtractTextFromXhtml(sanitized).Length == 0
                && !EpubContent.HasImageMarkup(sanitized))
                continue;

            var nodes = EpubHtmlStructure.BuildDocumentStructure(sanitized);

            if (nodes.Count == 0)
            {
                // Fallback: plain text.
                string chapterTitle = ExtractTitleFromXhtml(sanitized) ?? $"Chapter {index + 1}";
                builder.PushHeading(1, chapterTitle, null, null);

                string text = EpubContent.ExtractTextFromXhtml(sanitized);
                foreach (var paragraph in SplitDoubleNewline(text))
                {
                    string trimmed = paragraph.Trim();
                    if (trimmed.Length > 0)
                        builder.PushParagraph(trimmed, new List<TextAnnotation>(), null, null);
                }
                continue;
            }

            ConvertNodes(archive, builder, nodes, filePath, index);
        }

        return builder.Build();
    }

    /// <summary>Convert flat structure nodes into internal elements. Mirrors the mod.rs match loop.</summary>
    private static void ConvertNodes(
        ZipArchive archive, InternalDocumentBuilder builder, List<StructNode> nodes, string filePath, int index)
    {
        bool inList = false;

        foreach (var node in nodes)
        {
            var content = node.Content;

            // Close an open list when the current node is not a ListItem.
            if (inList && content.Which != NodeContent.Tag.ListItem)
            {
                builder.EndList();
                inList = false;
            }

            switch (content.Which)
            {
                // The blockquote container is recorded even though its contents are not inside it
                // (issue #127): upstream brackets the span the node claims as children, and this
                // walker's nodes are flat, so the quote opens and closes at once and the quoted
                // paragraph follows it as a sibling.
                case NodeContent.Tag.Quote:
                    builder.PushQuoteStart();
                    builder.PushQuoteEnd();
                    continue;

                case NodeContent.Tag.Heading:
                    builder.PushHeading(content.Level, content.Text ?? "", null, null);
                    CollectAnnotationUris(builder, node.Annotations, content.Text ?? "");
                    break;

                case NodeContent.Tag.Paragraph:
                    builder.PushParagraph(content.Text ?? "", node.Annotations, null, null);
                    CollectAnnotationUris(builder, node.Annotations, content.Text ?? "");
                    break;

                case NodeContent.Tag.ListItem:
                    if (!inList)
                    {
                        builder.PushList(false);
                        inList = true;
                    }
                    builder.PushListItem(content.Text ?? "", false, new List<TextAnnotation>(), null, null);
                    break;

                case NodeContent.Tag.Table:
                {
                    var grid = content.Grid ?? new TableGrid();
                    var cells = new List<List<string>>();
                    for (uint r = 0; r < grid.Rows; r++)
                    {
                        var rowCells = grid.Cells.Where(c => c.Row == r).Select(c => c.Content).ToList();
                        cells.Add(rowCells);
                    }
                    if (cells.Count > 0)
                        builder.PushTableFromCells(cells, null, null);
                    break;
                }

                case NodeContent.Tag.Code:
                    builder.PushCode(content.Text ?? "", content.Language, null, null);
                    break;

                case NodeContent.Tag.Formula:
                    builder.PushFormula(content.Text ?? "", null, null);
                    break;

                case NodeContent.Tag.Image:
                {
                    string? src = content.Src;
                    string? description = content.Description;

                    if (!string.IsNullOrEmpty(src))
                    {
                        builder.PushUri(new ExtractedUri
                        {
                            Url = src!,
                            Label = description,
                            Page = (uint)(index + 1),
                            Kind = UriKind.Image,
                        });
                    }

                    // Image src is relative to the XHTML file, not the manifest dir.
                    int slash = filePath.LastIndexOf('/');
                    string xhtmlDir = slash >= 0 ? filePath.Substring(0, slash) : "";

                    (byte[] Data, string Format)? imageData = null;
                    if (!string.IsNullOrEmpty(src)
                        && EpubContainer.TryResolvePath(xhtmlDir, src!, out var resolved, out _))
                    {
                        byte[] buf = EpubContainer.ReadBytesFromZip(archive, resolved.Path);
                        if (buf.Length > 0)
                            imageData = (buf, FormatFromSrc(src!));
                    }

                    if (imageData is { } data)
                    {
                        var image = new ExtractedImage
                        {
                            Data = data.Data,
                            Format = data.Format,
                            ImageIndex = 0,
                            PageNumber = (uint)(index + 1),
                            IsMask = false,
                            Description = description,
                        };
                        builder.PushImage(description, image, null, null);
                    }
                    else
                    {
                        // No image data — emit placeholder with sentinel index.
                        string textVal = description ?? "";
                        builder.PushElement(InternalElement.TextElement(ElementKind.Image(uint.MaxValue), textVal, 0));
                    }
                    break;
                }

                case NodeContent.Tag.DefinitionItem:
                    // Upstream carries an explicit arm here, added by its issue #127 after these
                    // fell into the catch-all and were dropped whole — the structure walker does
                    // produce them for `<dl>/<dt>/<dd>`. Only `DefinitionList` and `List`, which
                    // are containers, are skipped.
                    builder.PushDefinitionTerm(content.Term ?? "", null);
                    builder.PushDefinitionDescription(content.Definition ?? "", null);
                    break;

                // Group{heading_text}, DefinitionList, RawBlock, MetadataBlock, List, etc.:
                // skipped, matching the mod.rs `_ => {}` arm.
            }
        }

        if (inList)
            builder.EndList();
    }

    /// <summary>Collect hyperlink URIs from link annotations. Mirrors `collect_annotation_uris`.</summary>
    private static void CollectAnnotationUris(InternalDocumentBuilder builder, List<TextAnnotation> annotations, string text)
    {
        byte[]? utf8 = null;
        foreach (var ann in annotations)
        {
            if (ann.Kind.Which != AnnotationKind.Tag.Link) continue;
            string url = ann.Kind.Url ?? "";
            if (url.Length == 0) continue;

            string? label = null;
            if (ann.Start < ann.End)
            {
                utf8 ??= System.Text.Encoding.UTF8.GetBytes(text);
                if (ann.End <= (uint)utf8.Length)
                {
                    try
                    {
                        string slice = System.Text.Encoding.UTF8.GetString(utf8, (int)ann.Start, (int)(ann.End - ann.Start));
                        if (slice.Length > 0) label = slice;
                    }
                    catch (Exception) when (Cancel.NotCancelling)
                    {
                        label = null;
                    }
                }
            }

            builder.PushUri(new ExtractedUri
            {
                Url = url,
                Label = label,
                Page = null,
                Kind = ClassifyUri(url),
            });
        }
    }

    /// <summary>Mirrors `classify_uri`.</summary>
    private static XRay.Content.Types.UriKind ClassifyUri(string url)
    {
        if (url.StartsWith("mailto:", StringComparison.Ordinal)) return XRay.Content.Types.UriKind.Email;
        if (url.StartsWith('#')) return XRay.Content.Types.UriKind.Anchor;
        return XRay.Content.Types.UriKind.Hyperlink;
    }

    /// <summary>Extract the first h1/h2/h3 heading text. Mirrors `extract_title_from_xhtml`.</summary>
    private static string? ExtractTitleFromXhtml(string xhtml)
    {
        string sanitized = EpubContent.NormalizeXhtml(xhtml);
        System.Xml.Linq.XDocument doc;
        try
        {
            doc = Cancel.ParseXml(sanitized, System.Xml.Linq.LoadOptions.PreserveWhitespace);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            return null;
        }

        foreach (var node in doc.Descendants())
        {
            string tag = node.Name.LocalName.ToLowerInvariant();
            if (tag is "h1" or "h2" or "h3")
            {
                string trimmed = node.Value.Trim();
                if (trimmed.Length > 0) return trimmed;
            }
        }
        return null;
    }

    // Extension -> canonical image format token. Mirrors the mod.rs `match ext` blocks.
    private static string FormatFromExtension(string? ext) => (ext?.ToLowerInvariant()) switch
    {
        "jpg" or "jpeg" => "jpeg",
        "png" => "png",
        "gif" => "gif",
        "webp" => "webp",
        "svg" => "svg",
        "bmp" => "bmp",
        _ => "png",
    };

    private static string FormatFromPath(string path)
    {
        int dot = path.LastIndexOf('.');
        return FormatFromExtension(dot >= 0 ? path.Substring(dot + 1) : null);
    }

    private static string FormatFromSrc(string src)
    {
        // Rust uses `src.rsplit('.').next()` — the segment after the last '.'.
        int dot = src.LastIndexOf('.');
        return FormatFromExtension(dot >= 0 ? src.Substring(dot + 1) : src);
    }

    private static IEnumerable<string> SplitDoubleNewline(string text)
    {
        int start = 0;
        int idx;
        while ((idx = text.IndexOf("\n\n", start, StringComparison.Ordinal)) >= 0)
        {
            yield return text.Substring(start, idx - start);
            start = idx + 2;
        }
        yield return text.Substring(start);
    }
}
