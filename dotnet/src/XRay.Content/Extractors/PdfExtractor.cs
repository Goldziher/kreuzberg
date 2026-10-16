// Ported from crates/xberg/src/extractors/pdf/mod.rs (+ pdf/oxide/{text,metadata}.rs).
// Pure-managed PDF extractor: parses the document, walks pages, extracts text in
// reading order, and reads the info-dictionary metadata. No native dependencies,
// no OCR (image-only pages yield whatever native extraction finds).
//
// Backend lives under XRay.Content.Internal.Pdf (xref/trailer, objects, streams + filters,
// content-stream tokenizer, text-showing operators, font encoding/ToUnicode, page tree).
using XRay.Content.Core;
using XRay.Content.Internal.Archive;
using XRay.Content.Internal.Pdf;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>Native PDF extractor. Ports Rust `PdfExtractor` (native, non-OCR path).</summary>
public sealed class PdfExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/pdf" };

    public int Priority => 50;


    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        byte[] bytes = content.ToArray();

        var pdfOptions = config.Pdf ?? new PdfOptions();
        var passwords = pdfOptions.CandidatePasswords();

        PdfDocument pdf;
        try { pdf = PdfDocument.Open(bytes, passwords); }
        catch (Exception e) when (Cancel.NotCancelling) { throw new InvalidDataException($"pdf parse failed: {e.Message}", e); }

        // "Needs a password" is not "corrupt": a Standard-handler document that no supplied
        // password (nor the empty one) opened fails with a type of its own, so the caller can ask
        // for one instead of treating the file as broken.
        if (pdf.Encryption is { Supported: true } declared && pdf.Decryptor is null)
            throw new PdfPasswordRequiredException(passwords.Count > 0, declared);

        int pageCount = pdf.PageCount;
        if (pageCount == 0)
            throw new InvalidDataException("pdf has no readable pages");

        // Reject a document whose page count exceeds the configured ceiling before any per-page
        // work starts. Byte-size limits do not bound page count: a scanned page can compress to a
        // few kilobytes, so a file well under max_content_size can still hold thousands of pages.
        DocumentLimits.EnforcePageCount(pageCount, config.SecurityLimits);

        // Per-document wall-clock guard so pathological files cannot hang extraction. Scaled by
        // page count, so it is computed here rather than before the open: a fixed budget either
        // starves a large document or lets a small broken one spin for just as long.
        long deadline = config.Options.PdfDeadlineFromNow(pageCount);

        // --- Single per-page content pass: page text + font-metric segments ---
        // Content-stream parsing dominates cost, so parse each page exactly once and
        // derive both the assembled page text (plain/json) and the SegmentData used for
        // tables + heading structure (md/html) from the same spans.
        bool needsStructured = config.OutputFormat.Equals(OutputFormat.Markdown)
            || config.OutputFormat.Equals(OutputFormat.Djot)
            || config.OutputFormat.Equals(OutputFormat.Html);

        string nativeText = ExtractTextAndSegments(pdf, deadline, config.Options, out var pageSegments,
            out var pageWords, out var pagePaths);

        // --- Metadata ---
        var meta = PdfMetadataExtractor.Extract(pdf);

        // The outline is read once: the structure pipeline recovers headings from it, the metadata
        // lists it, and its destinations are URIs.
        List<PdfOutlineEntry> outline;
        try { outline = PdfBookmarks.ExtractOutlineEntries(pdf); }
        catch (Exception) when (Cancel.NotCancelling) { outline = new List<PdfOutlineEntry>(); }
        // Advisory: a document we cannot grade reports no scan evidence.
        var scanDetection = PdfScanDetect.Detect(pdf);

        // --- Tables: native → bordered → heuristic, each later tier folded into what the
        // earlier ones found (crates/xberg/src/extractors/pdf/extraction.rs).
        // Extracted regardless of output format — tables live in the result
        // independent of `content`.
        //
        // The tiers hand off by region and by content, not by page. Upstream skips the whole
        // page an earlier tier found anything on, which is right while a page holds one table
        // and wrong in two ways the moment it does not. A page carrying a three-column grid and
        // a two-column one — the shape every sectioned document lands in, where one section's
        // table ends and the next one's begins further down — loses the two-column table
        // entirely, because only the relaxed tier accepts two columns and it never runs there.
        // And a table ruled down only some of its columns is claimed by a ruled tier as the
        // narrow grid it can see, which then suppresses for the whole page the tier that could
        // have read the rest of it. Folding by region keeps the hand-off; keeping whichever
        // candidate carries more content over a shared region is what stops a partial reading
        // from shadowing a fuller one. Both are what upstream's own `prepare_emitted_tables`
        // does to the combined list, one pass later.
        var ruled = new List<RuledTable>();
        try { ruled.AddRange(ExtractRuledTables(pageWords, pagePaths, TableDetectionConfig.Strict())); }
        catch (Exception) when (Cancel.NotCancelling) { }
        try { FoldTier(ruled, ExtractRuledTables(pageWords, pagePaths, TableDetectionConfig.Bordered()), r => r.Table); }
        catch (Exception) when (Cancel.NotCancelling) { }

        // A table longer than a page arrives as a fragment per page, because every tier here
        // reads one page at a time. Joining them needs the ruling-line geometry, so it happens
        // while that is still to hand — and before the heuristic tier, whose tables have none.
        // Which pages the ruled tiers read is taken before the join, not after: a joined table
        // is filed under the page it starts on, and the pages its rest covers are just as much
        // claimed as that one. A continuation candidate claims nothing — it is not a table the
        // ruled tiers found, and letting it stand for one would silence the heuristic tier on
        // a page whose only ruled reading was a band too small to keep.
        var ruledPages = new HashSet<uint>(
            ruled.Where(r => !r.IsContinuationCandidate).Select(r => r.Table.PageNumber));
        var (tables, continuations) = PdfTableContinuation.Join(ruled);
        // The heuristic tier keeps upstream's page-level skip for *new* tables — it is the
        // loosest of the three, and letting it loose on a page the ruled tiers have already
        // read turns running prose into tables. What it may still do on such a page is replace
        // a ruled table that is only a part of what is there.
        try
        {
            FoldTier(
                tables,
                PdfTableReconstruct.ExtractHeuristicTables(
                    pageSegments, allowSingleColumn: false, skipPages: null, pagePaths),
                t => t,
                ruledPages);
        }
        catch (Exception) when (Cancel.NotCancelling) { }
        foreach (var table in tables) PdfTableNormalize.RepairConsistentlyMergedNumericColumn(table);

        // Upstream reduces this combined list before emitting it (`prepare_emitted_tables`):
        // it drops empty-markdown tables, collapses same-page boxes overlapping by more than
        // half the smaller one's area onto whichever candidate carries more content, and drops
        // exact repeats of a page's markdown. None of that is done here, and that is a measured
        // decision rather than an oversight. Both halves were ported and swept: the overlap
        // pass alone cost `ok` 363 -> 360, `json` 366 -> 364 and `tables` 377 -> 374, and the
        // empty-markdown drop plus the exact-repeat pass still cost `ok` 363 -> 361,
        // `json` 366 -> 364 and `tables` 377 -> 375.
        //
        // The reason is the pass that runs immediately BEFORE them upstream and is not ported:
        // `stitch_fragmented_tables` first joins a grid's vertically-adjacent same-column
        // fragments into one table. Upstream therefore reaches the reduction with far fewer
        // overlapping and repeating entries than the three tiers here produce, and running the
        // reduction on the unstitched list discards fragments the goldens keep. These belong
        // with the stitching pass, and only with it.

        // --- Build InternalDocument ---
        // For Markdown/Djot/HTML the Rust path pre-renders a *structured* document with
        // heading detection (pdf/structure/pipeline.rs). Plain/Json use the flat native
        // text split. Mirror that: structured elements only when the output format needs them.
        InternalDocument doc;
        InternalDocument? structured = null;
        if (needsStructured)
        {
            // The joined tables place and the fragments cover: a page holding the rest of a
            // table joined from the page before carries no table of its own, and its rows would
            // otherwise come back out as prose beside the table they are already in.
            var coverage = new List<XRay.Content.Types.Table>(tables);
            coverage.AddRange(continuations);
            try
            {
                structured = PdfStructure.Build(
                    pageSegments, ruledTables: tables, outlineEntries: outline, ruledCoverage: coverage);
            }
            catch (Exception) when (Cancel.NotCancelling) { structured = null; }
        }

        if (structured is not null && structured.Elements.Count > 0)
        {
            doc = structured;
        }
        else
        {
            doc = new InternalDocument("pdf");
            foreach (var paragraph in TextTransform.NormalizeLineEndings(nativeText).Split("\n\n"))
            {
                var trimmed = paragraph.Trim();
                if (trimmed.Length > 0)
                    doc.PushElement(InternalElement.TextElement(ElementKind.Paragraph, trimmed, 0));
            }
        }
        // Text repair belongs to the structure pipeline (Rust `pdf/structure/pipeline.rs`
        // runs it over the elements that pipeline assembles), not to the flat native-text
        // split: applying it there over-corrects text whose ligatures the Rust plain path
        // keeps, and measurably loses fixtures.
        if (structured is not null && ReferenceEquals(doc, structured))
        {
            foreach (var elem in doc.Elements)
            {
                if (elem.Text.Length == 0) continue;
                elem.Text = PdfTextRepair.Repair(elem.Text);
            }
        }

        doc.MimeType = mimeType;

        doc.Metadata = new Metadata
        {
            Title = meta.Title,
            Subject = meta.Subject,
            Authors = meta.Authors,
            Keywords = meta.Keywords,
            CreatedAt = meta.CreatedAt,
            ModifiedAt = meta.ModifiedAt,
            CreatedBy = meta.CreatedBy,
            Language = meta.Language,
            OcrUsed = false,
            Format = new FormatMetadata
            {
                FormatType = "pdf",
                Payload = new XRay.Content.Types.PdfMetadata
                {
                    PdfVersion = meta.PdfVersion,
                    Producer = meta.Producer,
                    IsEncrypted = meta.IsEncrypted,
                    Width = meta.Width,
                    Height = meta.Height,
                    PageCount = meta.PageCount,
                    ScannedConfidence = (float)scanDetection.Confidence,
                    ScannedPages = scanDetection.ScannedPageNumbers(PdfScanDetect.DefaultScannedMinConfidence),
                    RequiresPassword = meta.Encryption?.RequiresPassword,
                    AuthenticatedAs = meta.Encryption is { Supported: true } ea && ea.Authenticated != PdfAuthentication.None
                        ? ea.Authenticated.ToString().ToLowerInvariant() : null,
                    EncryptionAlgorithm = meta.Encryption?.Algorithm,
                    EncryptionKeyBits = meta.Encryption?.KeyBits,
                    Permissions = meta.Encryption is { } enc ? ToPermissions(enc.Permissions) : null,
                    Outline = outline.Count == 0 ? null : outline
                        .Select(o => new PdfOutlineItem { Title = o.Title, Level = o.Depth, Page = o.PageNumber })
                        .ToList(),
                },
            },
        };
        doc.Metadata.Additional["extraction_method"] =
            System.Text.Json.JsonSerializer.SerializeToElement("native");

        if (meta.CatalogVersion is not null)
            doc.Metadata.Additional["pdf_catalog_version"] =
                System.Text.Json.JsonSerializer.SerializeToElement(meta.CatalogVersion);
        if (meta.CustomInfo is { Count: > 0 })
            doc.Metadata.Additional["pdf_info"] =
                System.Text.Json.JsonSerializer.SerializeToElement(meta.CustomInfo);
        if (meta.XmpExtra is { Count: > 0 })
            doc.Metadata.Additional["xmp"] =
                System.Text.Json.JsonSerializer.SerializeToElement(meta.XmpExtra);

        // /PageLabels — one display label per page, index-aligned with the page structure.
        // Metadata has no field for it, so it rides in `additional`, as upstream does.
        try
        {
            if (PdfPageLabels.ExtractAll(pdf) is { } pageLabels)
                doc.Metadata.Additional["page_labels"] =
                    System.Text.Json.JsonSerializer.SerializeToElement(pageLabels);
        }
        catch (Exception) when (Cancel.NotCancelling) { }

        // Filled AcroForm values reach `content` only through the per-page widget splice,
        // which the structured path never sees; this puts back the ones no element carries.
        try { InjectUnrepresentedFormFieldElements(doc, PdfFormFields.Extract(pdf)); }
        catch (Exception) when (Cancel.NotCancelling) { }
        try
        {
            var typedFields = PdfFormModel.Extract(pdf);
            if (typedFields.Count > 0) doc.FormFields = typedFields;
        }
        catch (Exception) when (Cancel.NotCancelling) { }

        // The structured path assembles its own table list in element order, so its indices
        // are already the ones its `Table` elements point at; only a document that carries
        // none takes the detected ones.
        AttachUnrepresentedTables(doc, tables);
        // Plain text already spells out everything the reconstructed tables were built from,
        // so a table element there would render the same words twice. Every other shape
        // renders a table as a grid the flat text cannot express.
        bool documentIsStructured = structured is not null && ReferenceEquals(doc, structured);
        InjectUnrepresentedTableElements(
            doc, documentIsStructured || !config.OutputFormat.Equals(OutputFormat.Plain));

        // Bookmarks are links: external URLs as hyperlinks, internal destinations as anchors.
        foreach (var uri in PdfBookmarks.ExtractBookmarkUris(outline)) doc.PushUri(uri);

        if (pdfOptions.ExtractAnnotations)
            AttachAnnotations(doc, pdf, pageWords);
        if (pdfOptions.ExtractEmbeddedFiles)
            AttachEmbeddedFiles(doc, pdf, config);

        if (meta.IsEncrypted && pdf.Decryptor is null && nativeText.Length == 0)
            doc.ProcessingWarnings.Add(new ProcessingWarning
            {
                Source = "pdf",
                Message = "PDF is encrypted and could not be decrypted with the empty password; text unavailable.",
            });

        // A font whose /ToUnicode CMap is broken or missing still draws correctly, so the page
        // yields well-formed Latin words that mean nothing. Nothing upstream can fail on that —
        // the codes are letters — so the only place it can be caught is the finished text.
        if (Internal.Text.UndecodedTextDetection.Diagnose(nativeText) is { } undecoded)
            doc.ProcessingWarnings.Add(new ProcessingWarning { Source = "pdf", Message = undecoded });

        // The growth guard runs over the finished elements rather than the input: a PDF's own
        // byte count says nothing about how much text it decompresses to, so the total is only
        // knowable once the pages are read.
        {
            var budget = SecurityBudget.FromConfig(config);
            foreach (var elem in doc.Elements)
                budget.AccountText(System.Text.Encoding.UTF8.GetByteCount(elem.Text));
        }

        return doc;
    }

    private static PdfPermissions ToPermissions(PdfPermissionFlags f) => new()
    {
        Raw = f.Raw, Print = f.Print, Modify = f.Modify, Copy = f.Copy, Annotate = f.Annotate,
        FillForms = f.FillForms, ExtractForAccessibility = f.ExtractForAccessibility,
        Assemble = f.Assemble, PrintHighQuality = f.PrintHighQuality,
    };

    /// <summary>
    /// Read the page annotations onto the document, and surface the links among them as URIs.
    /// A failure costs the annotations, not the document.
    /// </summary>
    private static void AttachAnnotations(InternalDocument doc, PdfDocument pdf, List<List<TableSpan>> pageWords)
    {
        try
        {
            var annotations = PdfAnnotations.Extract(pdf, pageWords, doc.ProcessingWarnings);
            if (annotations.Count == 0) return;
            doc.Annotations = annotations;

            foreach (var a in annotations)
            {
                if (a.AnnotationType != PdfAnnotationType.Link) continue;
                string? url = a.Content ?? a.Destination;
                if (url is null) continue;
                var kind = url.StartsWith('#') ? UriKind.Anchor
                    : url.StartsWith("mailto:", StringComparison.Ordinal) ? UriKind.Email
                    : UriKind.Hyperlink;
                doc.PushUri(new ExtractedUri { Url = url, Label = url, Page = a.PageNumber, Kind = kind });
            }
        }
        catch (Exception e) when (Cancel.NotCancelling)
        {
            doc.ProcessingWarnings.Add(new ProcessingWarning
            {
                Source = "pdf_annotations",
                Message = $"annotation extraction failed: {e.Message}",
            });
        }
    }

    /// <summary>
    /// List the document's attachments and extract each one this package reads through the normal
    /// pipeline into <c>Children</c>, one archive level deeper and under the same size, ratio and
    /// depth limits an archive's members get.
    /// </summary>
    private static void AttachEmbeddedFiles(InternalDocument doc, PdfDocument pdf, ExtractionConfig config)
    {
        List<PdfEmbeddedFile> files;
        try { files = PdfEmbeddedFiles.Collect(pdf, config, doc.ProcessingWarnings); }
        catch (Exception e) when (Cancel.NotCancelling)
        {
            doc.ProcessingWarnings.Add(new ProcessingWarning
            {
                Source = "pdf_embedded_files",
                Message = $"embedded file extraction failed: {e.Message}",
            });
            return;
        }
        if (files.Count == 0) return;

        var options = config.Pdf ?? new PdfOptions();
        var attachments = new List<ExtractedAttachment>(files.Count);
        var children = new List<ArchiveEntry>();
        foreach (var file in files)
        {
            Cancel.ThrowIfRequested();
            var attachment = new ExtractedAttachment
            {
                Name = file.Name,
                MimeType = file.MimeType,
                Size = file.Data.LongLength,
                CompressedSize = file.CompressedSize,
                Description = file.Description,
                Relationship = file.Relationship,
                Created = file.Created,
                Modified = file.Modified,
                Source = file.Source,
                PageNumber = file.PageNumber,
                SkippedReason = file.SkippedReason,
                Data = options.AttachEmbeddedFileBytes && file.SkippedReason is null ? file.Data : null,
            };
            attachments.Add(attachment);
            if (file.SkippedReason is not null || file.Data.Length == 0) continue;

            string? mime = ChildMime(file);
            if (mime is null || mime == Mime.OctetStream) continue;
            var entry = ArchiveDocument.TryExtractChild(
                file.Name, file.Data, mime, config, "pdf_embedded_files", doc.ProcessingWarnings);
            if (entry is not null) children.Add(entry);
            else attachment.SkippedReason ??= "not extracted (see processing warnings)";
        }
        doc.Attachments = attachments;
        if (children.Count > 0) (doc.Children ??= new List<ArchiveEntry>()).AddRange(children);
    }

    /// <summary>
    /// The type to extract an attachment as: what its bytes say when that is specific, else the
    /// filespec's own claim, else its name.
    /// </summary>
    private static string? ChildMime(PdfEmbeddedFile file)
    {
        string? sniffed = Mime.DetectMimeTypeFromBytes(file.Data);
        bool specific = sniffed is not null && sniffed != Mime.OctetStream && sniffed != "text/plain";
        if (specific) return sniffed;
        if (file.MimeType is { Length: > 0 } declared && declared != Mime.OctetStream) return declared;
        return sniffed ?? Mime.DetectMimeType(file.Name, checkExists: false);
    }

    /// <summary>
    /// Fraction of the smaller box two tables must share before they are taken to be the same
    /// region seen by two tiers.
    /// </summary>
    private const double TableRegionOverlapRatio = 0.5;

    /// <summary>
    /// Largest share of a candidate's area an earlier table may already cover and still count
    /// as a partial reading of it rather than a competing one.
    /// </summary>
    private const double TablePartialReadingRatio = 0.6;

    /// <summary>
    /// Fold a later tier's tables into what the earlier tiers found. A candidate over fresh
    /// ground is kept. A candidate over ground already claimed is dropped — the earlier tier is
    /// the more trustworthy — unless what is there is only a *part* of it: a table ruled down
    /// two of its eight columns is claimed by a ruled tier as that narrow grid, and the tier
    /// that can read all eight should not lose to it. Reaching over much more of the page and
    /// reading more of it is what separates that from an ordinary disagreement, where the
    /// earlier tier keeps the region.
    /// </summary>
    private static void FoldTier<T>(
        List<T> kept, IEnumerable<T> candidates, Func<T, XRay.Content.Types.Table> table,
        HashSet<uint>? supersedeOnlyOn = null)
    {
        foreach (var entry in candidates)
        {
            var candidate = table(entry);
            var overlapping = new List<int>();
            for (int i = 0; i < kept.Count; i++)
                if (SameRegion(table(kept[i]), candidate)) overlapping.Add(i);

            if (overlapping.Count == 0)
            {
                if (supersedeOnlyOn is null || !supersedeOnlyOn.Contains(candidate.PageNumber))
                    kept.Add(entry);
                continue;
            }

            double area = BoxArea(candidate);
            int content = ContentCells(candidate);
            bool supersedes = area > 0;
            foreach (int i in overlapping)
                if (ContentCells(table(kept[i])) >= content
                    || BoxArea(table(kept[i])) >= area * TablePartialReadingRatio)
                {
                    supersedes = false;
                    break;
                }
            if (!supersedes) continue;

            for (int i = overlapping.Count - 1; i >= 0; i--) kept.RemoveAt(overlapping[i]);
            kept.Add(entry);
        }
    }

    private static double BoxArea(XRay.Content.Types.Table table) =>
        table.BoundingBox is { } b ? Math.Max((b.X1 - b.X0) * (b.Y1 - b.Y0), 0.0) : 0.0;

    /// <summary>Whether two tables are two readings of one region of one page.</summary>
    private static bool SameRegion(XRay.Content.Types.Table a, XRay.Content.Types.Table b)
    {
        if (a.PageNumber != b.PageNumber) return false;
        if (a.BoundingBox is not { } x || b.BoundingBox is not { } y) return false;
        double w = Math.Min(x.X1, y.X1) - Math.Max(x.X0, y.X0);
        double h = Math.Min(x.Y1, y.Y1) - Math.Max(x.Y0, y.Y0);
        if (w <= 0 || h <= 0) return false;
        double smaller = Math.Min(
            Math.Max((x.X1 - x.X0) * (x.Y1 - x.Y0), 0.0),
            Math.Max((y.X1 - y.X0) * (y.Y1 - y.Y0), 0.0));
        return smaller <= 0 || w * h / smaller >= TableRegionOverlapRatio;
    }

    /// <summary>How much of a region a table actually read: its non-empty cells.</summary>
    private static int ContentCells(XRay.Content.Types.Table table)
    {
        int count = 0;
        foreach (var row in table.Cells)
            foreach (string cell in row)
                if (cell.Trim().Length > 0) count++;
        return count;
    }

    // Single per-page pass: parse each page's content stream once, then derive both the
    // assembled page text (returned, joined by blank lines) and the font-metric
    /// <summary>Run one ruling-line tier over every page, keeping its grid geometry.</summary>
    private static List<RuledTable> ExtractRuledTables(
        List<List<TableSpan>> pageWords, List<List<PdfPath>> pagePaths, TableDetectionConfig config)
    {
        var tables = new List<RuledTable>();
        for (int i = 0; i < pageWords.Count && i < pagePaths.Count; i++)
        {
            uint pageNumber = (uint)(i + 1);
            try
            {
                tables.AddRange(
                    PdfSpatialTables.DetectPageRuledTables(pageWords[i], pagePaths[i], pageNumber, config));
            }
            catch (Exception) when (Cancel.NotCancelling) { }
        }
        return tables;
    }

    /// <summary>
    /// Page-text cleanup: control-character repair, then a markup pass for the pages that
    /// carry raw HTML.
    /// </summary>
    /// <remarks>
    /// Web-to-PDF converters sometimes leave the source markup in the text layer, where the
    /// tags read as words. Such a page is converted as HTML rather than used as-is.
    /// </remarks>
    internal static string ApplyTextCleanup(string text)
    {
        string cleaned = PdfPageText.FixControlChars(text);
        return ContainsHtmlMarkup(cleaned) ? Internal.Html.HtmlToMarkdown.Convert(cleaned) : cleaned;
    }

    /// <summary>Whether the page text carries embedded HTML markup.</summary>
    internal static bool ContainsHtmlMarkup(string text) =>
        text.Contains('<')
        && (text.Contains("</p>", StringComparison.Ordinal)
            || text.Contains("<br", StringComparison.Ordinal)
            || text.Contains("<p>", StringComparison.Ordinal)
            || text.Contains("<div", StringComparison.Ordinal)
            || text.Contains("<span", StringComparison.Ordinal)
            || text.Contains("<table", StringComparison.Ordinal)
            || text.Contains("<a ", StringComparison.Ordinal)
            || text.Contains("/>", StringComparison.Ordinal));

    /// <summary>Share of a table's cell tokens that must already be in the element stream
    /// before the table counts as represented.</summary>
    private const double MIN_TABLE_TOKEN_REPRESENTATION = 0.90;

    /// <summary>Cell-token count below which the containment check abstains: on a handful of
    /// tokens an incidental overlap with unrelated prose is likely, and injecting a duplicate
    /// is cheaper than dropping a real table.</summary>
    private const int MIN_TABLE_TOKENS_FOR_CONTAINMENT = 8;

    private static readonly char[] AsciiPunctuation =
        "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~".ToCharArray();

    /// <summary>Words with edge punctuation and case removed, so whitespace, line wrapping
    /// and trailing punctuation do not make the same text look different.</summary>
    internal static IEnumerable<string> NormalizedPdfTokens(string text)
    {
        foreach (var word in text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
        {
            string trimmed = word.Trim(AsciiPunctuation).ToLowerInvariant();
            if (trimmed.Length > 0) yield return trimmed;
        }
    }

    /// <summary>
    /// Surface filled AcroForm values that no element already spells out, one paragraph per
    /// field, as <c>"{full name}: {value}"</c>.
    /// </summary>
    /// <remarks>
    /// The plain-text path splices widget values into the page text before it is chopped into
    /// paragraphs, so its elements already carry them and the containment test skips them; the
    /// structured path is built from span segments that never see that splice.
    /// </remarks>
    internal static void InjectUnrepresentedFormFieldElements(
        InternalDocument doc, List<PdfAcroFormField> formFields)
    {
        foreach (var field in formFields)
        {
            string? value = field.Value;
            if (string.IsNullOrEmpty(value)) continue;
            if (doc.Elements.Any(element => element.Text.Contains(value, StringComparison.Ordinal))) continue;
            string displayName = field.FullName.Length == 0 ? field.Name : field.FullName;
            doc.PushElement(InternalElement.TextElement(ElementKind.Paragraph, $"{displayName}: {value}", 0));
        }
    }

    /// <summary>Give a document that has no tables of its own the ones detection found.</summary>
    internal static void AttachUnrepresentedTables(InternalDocument doc, List<XRay.Content.Types.Table> tables)
    {
        if (doc.Tables.Count != 0) return;
        foreach (var table in tables) doc.PushTable(table);
    }

    /// <summary>Tokens the element stream already renders, as a consumable multiset.</summary>
    internal static Dictionary<string, int> ElementTokenMultiset(InternalDocument doc)
    {
        var represented = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var element in doc.Elements)
            foreach (var token in NormalizedPdfTokens(element.Text))
                represented[token] = represented.TryGetValue(token, out var n) ? n + 1 : 1;
        return represented;
    }

    /// <summary>
    /// Whether a table's cell text is already carried by the element stream.
    /// </summary>
    /// <remarks>
    /// Accounting is multiset based: each text occurrence backs at most one cell occurrence,
    /// and the occurrences are consumed only when the table is judged represented, so one
    /// table cannot mask the next. Both bailouts err toward injecting, because a duplicated
    /// table costs precision while a dropped one costs content.
    /// </remarks>
    internal static bool TableIsRepresented(XRay.Content.Types.Table table, Dictionary<string, int> represented)
    {
        var tableTokens = new Dictionary<string, int>(StringComparer.Ordinal);
        int tableTokenCount = 0;
        foreach (var row in table.Cells)
            foreach (var cell in row)
                foreach (var token in NormalizedPdfTokens(cell))
                {
                    tableTokens[token] = tableTokens.TryGetValue(token, out var n) ? n + 1 : 1;
                    tableTokenCount++;
                }
        if (tableTokenCount < MIN_TABLE_TOKENS_FOR_CONTAINMENT) return false;

        int matched = 0;
        foreach (var (token, count) in tableTokens)
            matched += Math.Min(count, represented.TryGetValue(token, out var have) ? have : 0);
        if ((double)matched / tableTokenCount < MIN_TABLE_TOKEN_REPRESENTATION) return false;

        foreach (var (token, count) in tableTokens)
            if (represented.TryGetValue(token, out var remaining))
                represented[token] = Math.Max(0, remaining - count);
        return true;
    }

    internal static void InjectUnrepresentedTableElements(InternalDocument doc, bool allowInjection)
    {
        if (!allowInjection) return;
        foreach (var element in doc.Elements)
            if (element.Kind.Tag == ElementKindTag.Table) return;

        // The `Table`-element guard above only ever fires on the structured path; the flat
        // path builds nothing but paragraphs, so every detected table would be injected on
        // top of native text that already contains the words it was reconstructed from.
        var represented = ElementTokenMultiset(doc);
        for (int tableIndex = 0; tableIndex < doc.Tables.Count; tableIndex++)
        {
            if (TableIsRepresented(doc.Tables[tableIndex], represented)) continue;
            doc.PushElement(InternalElement.TextElement(ElementKind.Table((uint)tableIndex), "", 0));
        }
    }



    // SegmentData grid (out param) used for tables and heading structure. Mirrors Rust
    // `oxide::text::extract_text` + `oxide::hierarchy::extract_all_segments` sharing spans.
    internal static string ExtractTextAndSegments(
        PdfDocument pdf, long deadline, XRayOptions options,
        out List<List<SegmentData>> pageSegments,
        out List<List<TableSpan>> pageWords, out List<List<PdfPath>> pagePaths)
    {
        int pageCount = pdf.PageCount;
        pageSegments = new List<List<SegmentData>>(pageCount);
        pageWords = new List<List<TableSpan>>(pageCount);
        pagePaths = new List<List<PdfPath>>(pageCount);
        var sb = new System.Text.StringBuilder();

        for (int i = 0; i < pageCount; i++)
        {
            // Match the previous text path: once the wall-clock guard trips, stop
            // emitting pages entirely rather than appending empty tails.
            if (DateTime.UtcNow.Ticks > deadline) break;

            string pageText = "";
            List<SegmentData> segs = new();
            List<TableSpan> words = new();
            List<PdfPath> paths = new();
            try
            {
                byte[] contentBytes = pdf.GetPageContent(i);
                if (contentBytes.Length != 0)
                {
                    var resources = pdf.Resolve(pdf.Pages[i].Get("Resources")).AsDict();
                    var extractor = new PdfContentExtractor(pdf, deadline);
                    var spans = extractor.Extract(contentBytes, resources);
                    var (mbLlx, mbLly, mbUrx, mbUry) = pdf.GetPageMediaBox(i);
                    double pageWidth = Math.Abs(mbUrx - mbLlx);
                    List<PdfPageText.LineSeg> lines;
                    List<XRay.Content.Internal.PdfOxide.OxTextSpan>? structureSpans = null;
                    List<XRay.Content.Internal.PdfOxide.OxTextSpan>? wordSpans = null;
                    float structurePageWidth = 0f, structurePageHeight = 0f;
                    if (options.UsePortedPdfSpans)
                    {
                        // The ported pipeline's own assembler consumes the ported spans directly:
                        // they already arrive column-aware ordered, deduplicated and merged, which
                        // is exactly the shape upstream's `assemble_page_text` receives.
                        var oxResult = XRay.Content.Internal.PdfOxide.Text.OxPageExtractor.ExtractPage(pdf, i);
                        var oxPage = oxResult.Text;
                        var oxSpans = oxPage.Spans;
                        wordSpans = oxResult.WordSpans;
                        var assembly = XRay.Content.Internal.PdfOxide.Text.OxPageAssembler.Assemble(
                            oxSpans, (float)pageWidth);
                        pageText = assembly.Text;
                        lines = PdfPageText.LineSegsFromOx(assembly.Lines);
                        // Bridged AFTER assembly, so the word grid sees the column repairs' order.
                        spans = OxSpanBridge.ToPdfSpans(oxSpans);
                        // The structure pipeline runs its own span pipeline over the same page —
                        // see PdfOxideSegments — so it keeps the spans whole rather than the
                        // assembler's lines.
                        structureSpans = oxResult.HierarchySpans;
                        structurePageWidth = oxPage.PageWidth;
                        structurePageHeight = oxPage.PageHeight;
                    }
                    else
                    {
                        (pageText, lines) = PdfPageText.AssembleWithLines(spans, pageWidth);
                    }
                    // The older interpreter still runs either way: its `Paths` are what the
                    // ruling-line table tiers read, and nothing in the ported pipeline collects
                    // those yet.
                    segs = structureSpans is not null
                        ? PdfOxideSegments.FromPage(structureSpans, structurePageWidth, structurePageHeight)
                        : PdfStructure.SegmentsFromLines(lines);
                    // The detector is fed words, not show-operator runs (upstream calls
                    // `extract_words`), and those come off their own pipeline: the word
                    // path's spans are post-processed and ordered by `page_reading_order`,
                    // where the text and hierarchy paths see only the off-page drop and a
                    // sort. The per-glyph geometry the clustering needs exists only on the
                    // ported extractor's spans.
                    words = wordSpans is not null
                        ? PdfSpatialTables.WordsFromOxSpans(wordSpans)
                        : PdfSpatialTables.SpansToWords(spans);
                    paths = extractor.Paths;
                }
                // AcroForm: interactive text-field values are stored as the widget's /V and are
                // not drawn into the content stream when no appearance stream exists. pdf_oxide
                // surfaces them, appended after the page's content text. Mirror that here.
                var formValues = CollectWidgetTextValues(pdf, i);
                pageText = AppendMissingWidgetValues(pageText, formValues);
            }
            catch (Exception) when (Cancel.NotCancelling) { pageText = ""; segs = new(); words = new(); paths = new(); }

            pageSegments.Add(segs);
            pageWords.Add(words);
            pagePaths.Add(paths);
            if (i > 0) sb.Append("\n\n");
            sb.Append(ApplyTextCleanup(pageText));
        }
        return sb.ToString();
    }

    /// <summary>
    /// Append the widget values the page's own content stream does not already carry.
    /// </summary>
    /// <remarks>
    /// A flattened form has its field values rendered into the content stream as ordinary text,
    /// so appending them again would print each one twice. The containment test is a plain
    /// substring match: the rendered appearance text and the widget's <c>/V</c> string match
    /// verbatim in the common case, and suppressing a value that happens to be a substring of
    /// surrounding prose is the cheaper mistake.
    /// </remarks>
    internal static string AppendMissingWidgetValues(string text, List<string> values)
    {
        foreach (var value in values)
        {
            if (text.Contains(value, StringComparison.Ordinal)) continue;
            if (text.Length > 0 && !text.EndsWith('\n')) text += "\n";
            text += value;
        }
        return text;
    }

    // Collect the /V of every Widget annotation on one page, top to bottom. /V may be inherited
    // via the /Parent field chain. De-duplicated by fully-qualified field name so widgets that
    // share a parent field are not counted twice.
    private static List<string> CollectWidgetTextValues(PdfDocument pdf, int pageIndex)
    {
        var values = new List<(double MidY, string Value)>();
        var annots = pdf.Resolve(pdf.Pages[pageIndex].Get("Annots")).AsArray();
        if (annots is null) return new List<string>();

        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var a in annots.Items)
        {
            var widget = pdf.Resolve(a).AsDict();
            if (widget is null) continue;
            if (pdf.Resolve(widget.Get("Subtype")).AsName() != "Widget") continue;

            string? raw = null;
            var names = new List<string>();
            var node = widget;
            for (int guard = 0; node is not null && guard < 32; guard++)
            {
                raw ??= AnnotationStringValue(pdf.Resolve(node.Get("V")));
                var t = pdf.Resolve(node.Get("T")).AsStringBytes();
                if (t is not null) names.Insert(0, PdfMetadataExtractor.DecodePdfString(t) ?? "");
                node = pdf.Resolve(node.Get("Parent")).AsDict();
            }

            if (raw is null) continue;                          // no value set
            string value = raw.Trim();
            if (value.Length == 0) continue;

            string key = names.Count > 0 ? string.Join(".", names) : value;
            if (!seen.Add(key)) continue;

            // Values are appended after all content-stream text, so the only reading order left
            // to preserve is the widgets' own: nearest the top of the page first.
            var rect = pdf.Resolve(widget.Get("Rect")).AsArray();
            double midY = rect is { Items.Count: >= 4 }
                && pdf.Resolve(rect.Items[1]).AsNumber() is { } y0
                && pdf.Resolve(rect.Items[3]).AsNumber() is { } y1
                ? (y0 + y1) / 2.0
                : double.NegativeInfinity;
            values.Add((midY, value));
        }

        // Stable, so widgets sharing a row keep the order the /Annots array gave them.
        return values.OrderByDescending(v => v.MidY).Select(v => v.Value).ToList();
    }

    /// <summary>
    /// An annotation entry read as the string the widget splice appends.
    /// </summary>
    /// <remarks>
    /// Deliberately not the text-string decoding of ISO 32000-1 §7.9.2.2: the annotation layer
    /// reads the bytes as UTF-8 and replaces what does not decode, so a UTF-16BE value reaches
    /// the page text with its byte-order mark as replacement characters. The document-level
    /// AcroForm model (<see cref="PdfFormFields"/>) is the one that decodes properly, and the
    /// two disagreeing is what leaves a UTF-16 value unrepresented for the injection pass.
    /// </remarks>
    private static string? AnnotationStringValue(PdfObject? value) => value switch
    {
        PdfString s => System.Text.Encoding.UTF8.GetString(s.Bytes),
        PdfName n => n.Value,
        PdfNumber { IsInteger: true } n => ((long)n.Value).ToString(System.Globalization.CultureInfo.InvariantCulture),
        PdfNumber n => n.Value.ToString("R", System.Globalization.CultureInfo.InvariantCulture),
        _ => null,
    };
}
