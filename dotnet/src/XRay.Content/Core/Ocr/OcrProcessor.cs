using XRay.Content.Types;

namespace XRay.Content.Core.Ocr;

/// <summary>
/// Runs the optional OCR pass over a finished <see cref="InternalDocument"/>, before it is
/// rendered.
/// </summary>
/// <remarks>
/// <para>
/// It runs before rendering, unlike <see cref="QrPostProcessor"/>, and that is the whole reason
/// the text lands where it belongs. A pass over the rendered string could only append; working
/// on the element stream, an image's text can be inserted immediately after the image element
/// that produced it, so every renderer places it inline for free and the structured output keeps
/// the association.
/// </para>
/// <para>
/// OCR is additive here. Native text is never replaced, a page that already has text keeps it,
/// and any failure — a missing checkpoint, an undecodable image, a recognition that outran its
/// budget — is recorded as a processing warning while the document is returned intact. A
/// document is not a failure for lacking OCR.
/// </para>
/// </remarks>
internal static class OcrProcessor
{
    private const string WarningSource = "ocr";

    /// <summary>
    /// Recognise what the configured mode asks for, inserting the results into the document.
    /// </summary>
    /// <param name="doc">The extracted document, modified in place.</param>
    /// <param name="content">The original file's bytes, for the modes that rasterise pages.</param>
    /// <param name="mimeType">The document's MIME type.</param>
    /// <param name="config">Extraction settings, including <see cref="OcrOptions"/>.</param>
    /// <param name="engineFactory">
    /// Builds the recognizer. Injected so the flow can be tested without a multi-gigabyte
    /// checkpoint; production passes null and gets <see cref="PaddleOcrEngine"/>.
    /// </param>
    public static void Process(
        InternalDocument doc,
        ReadOnlySpan<byte> content,
        string mimeType,
        ExtractionConfig config,
        Func<OcrOptions, IOcrEngine>? engineFactory = null)
    {
        var configured = config.Ocr;
        if (configured is null || configured.Mode == OcrMode.Disabled) return;

        // Resolved once, here, because this is the only place that can see both the OCR settings
        // and the format the document is being rendered to.
        var options = configured.WithTextFormat(ResolveTextFormat(configured, config.OutputFormat));

        // Work out what there is to do before loading any weights: a document with no scanned
        // page and no image must not pay for a checkpoint it will not use.
        var scannedPages = mimeType == PdfMime ? ScannedPageNumbers(doc) : Array.Empty<uint>();
        bool wantsImages = options.Mode == OcrMode.AllImages;
        var imageTargets = wantsImages ? ImageTargets(doc, options) : Array.Empty<int>();
        if (scannedPages.Length == 0 && imageTargets.Length == 0) return;

        byte[]? pdfBytes = scannedPages.Length > 0 ? content.ToArray() : null;

        IOcrEngine? engine = null;
        try
        {
            engine = (engineFactory ?? (o => new PaddleOcrEngine(o)))(options);
            int recognized = 0;

            if (pdfBytes is not null)
                recognized += RecognizeScannedPages(doc, pdfBytes, scannedPages, options, engine);

            if (imageTargets.Length > 0)
                recognized += RecognizeImages(doc, imageTargets, options, engine);

            if (recognized > 0)
            {
                doc.Metadata.OcrUsed = true;
                // `mixed` rather than `ocr`: the native text is still there, and a consumer that
                // treats the whole document as machine-read would be wrong about most of it.
                doc.Metadata.Additional["extraction_method"] =
                    System.Text.Json.JsonSerializer.SerializeToElement("mixed");
            }
        }
        catch (OcrUnavailableException e)
        {
            Warn(doc, e.Message);
        }
        finally
        {
            engine?.Dispose();
        }
    }

    private const string PdfMime = "application/pdf";

    /// <summary>
    /// The pages a scan detector flagged, read off the PDF metadata the extractor already
    /// produced rather than re-deriving them.
    /// </summary>
    private static uint[] ScannedPageNumbers(InternalDocument doc) =>
        doc.Metadata.Format?.Payload is PdfMetadata pdf && pdf.ScannedPages is { Count: > 0 } pages
            ? pages.ToArray()
            : Array.Empty<uint>();

    /// <summary>
    /// Which images are worth recognising: those that carry bytes, are the right size to hold
    /// text, and fall inside the per-document ceiling.
    /// </summary>
    private static int[] ImageTargets(InternalDocument doc, OcrOptions options)
    {
        var targets = new List<int>();
        for (int i = 0; i < doc.Images.Count && targets.Count < options.MaxImages; i++)
        {
            if (!IsWorthRecognising(doc.Images[i], options)) continue;
            targets.Add(i);
        }
        return targets.ToArray();
    }

    /// <summary>
    /// Whether one image is the right size to be worth reading.
    /// </summary>
    /// <remarks>
    /// Both ends matter and for different reasons. Below the floor there is nothing to read: an
    /// icon, a bullet, a separator rule and a spacer GIF are all images, and recognising them
    /// costs time and yields noise. Above the ceiling the image is decoded in full — into a
    /// buffer proportional to its pixel count — before the recognizer downsamples it to its own
    /// budget, so one poster-sized scan can cost more memory than the whole rest of the document.
    /// <para>
    /// Dimensions are advisory: several extractors record none, and an image of unknown size is
    /// recognised rather than skipped, because dropping those would silently disable the mode for
    /// the formats that do not measure their images. <see cref="OcrOptions.MaxImageBytes"/> is
    /// what bounds those, since the encoded length is always known.
    /// </para>
    /// </remarks>
    private static bool IsWorthRecognising(ExtractedImage image, OcrOptions options)
    {
        if (image.Data.Length == 0) return false;
        if (image.Data.Length > options.MaxImageBytes) return false;

        if (image.Width is not { } width || image.Height is not { } height) return true;

        long pixels = (long)width * height;
        if (pixels < options.MinImagePixels || pixels > options.MaxImagePixels) return false;
        return Math.Min(width, height) >= options.MinImageDimension;
    }

    /// <summary>
    /// What <see cref="OcrTextFormat.Auto"/> means for a given output format: markdown where the
    /// document is being rendered as markup, plain lines everywhere else.
    /// </summary>
    /// <remarks>
    /// HTML counts as markup because its renderer reaches the recognised text through the same
    /// markdown AST. Djot counts because its renderer emits the text as a verbatim block, so
    /// markup survives it; its own syntax differs from markdown in places, but markdown is the
    /// closer of the two shapes on offer.
    /// </remarks>
    private static OcrTextFormat ResolveTextFormat(OcrOptions options, OutputFormat output)
    {
        if (options.TextFormat != OcrTextFormat.Auto) return options.TextFormat;

        return output.Which switch
        {
            OutputFormat.Kind.Markdown or OutputFormat.Kind.Html or OutputFormat.Kind.Djot
                => OcrTextFormat.Markdown,
            _ => OcrTextFormat.PlainText,
        };
    }

    /// <summary>
    /// Element attribute naming the shape of a recognised text element, read by the markdown
    /// renderer. Shared with <c>ComrakBridge</c>, which is the only reader.
    /// </summary>
    internal const string MarkdownAttribute = "ocr_format";

    /// <summary>
    /// Rasterise each flagged page and append what it says, as a page-level OCR element.
    /// </summary>
    /// <remarks>
    /// Appended rather than inserted: a scanned page's native elements are the little text the
    /// page had — often none, sometimes a header the producer left behind — and there is no
    /// element to attach a whole page's recognition to. The page number on the element is what
    /// relates it to its page.
    /// </remarks>
    private static int RecognizeScannedPages(
        InternalDocument doc, byte[] pdfBytes, uint[] pages, OcrOptions options, IOcrEngine engine)
    {
        List<(uint Page, byte[] Png)> rendered;
        try
        {
            rendered = OcrPageRasterizer.Render(pdfBytes, pages, options.Dpi);
        }
        catch (Exception e) when (e is not OcrUnavailableException && Cancel.NotCancelling)
        {
            Warn(doc, $"the scanned pages could not be rasterised for OCR: {e.Message}");
            return 0;
        }

        int recognized = 0;
        foreach (var (page, png) in rendered)
        {
            Cancel.ThrowIfRequested();
            if (!TryRecognize(doc, engine, png, options, $"page {page}", out var result)) break;
            if (!result.HasText) continue;

            var elements = ElementsFor(doc, result.Text, OcrElementLevel.Page, page, options);
            if (elements.Count == 0) continue;
            doc.Elements.AddRange(elements);
            recognized++;
        }
        return recognized;
    }

    /// <summary>
    /// Recognise each image and put its text immediately after the element that references it,
    /// so it reads inline.
    /// </summary>
    /// <remarks>
    /// An image the element stream never references — several extractors collect images
    /// separately from the text — has nothing to sit after, so its text is appended instead of
    /// being dropped.
    /// </remarks>
    private static int RecognizeImages(
        InternalDocument doc, int[] targets, OcrOptions options, IOcrEngine engine)
    {
        // Collected first and inserted afterwards: inserting while scanning would invalidate the
        // indices the scan is still walking.
        var insertions = new List<(int AfterElement, List<InternalElement> Elements)>();
        var appended = new List<InternalElement>();
        int appendedImages = 0;

        foreach (int imageIndex in targets)
        {
            Cancel.ThrowIfRequested();
            var image = doc.Images[imageIndex];
            if (!TryRecognize(doc, engine, image.Data, options, $"image {imageIndex}", out var result)) break;
            if (!result.HasText) continue;

            int host = doc.Elements.FindIndex(e =>
                e.Kind.Tag == ElementKindTag.Image && e.Kind.ImageIndex == (uint)imageIndex);
            var elements = ElementsFor(
                doc, result.Text, OcrElementLevel.Block, host >= 0 ? doc.Elements[host].Page : null, options);
            if (elements.Count == 0) continue;

            if (host >= 0)
            {
                insertions.Add((host, elements));
            }
            else
            {
                appended.AddRange(elements);
                appendedImages++;
            }
        }

        // Back to front, so an earlier insertion cannot shift a later one's position.
        foreach (var (after, elements) in insertions.OrderByDescending(i => i.AfterElement))
            doc.Elements.InsertRange(after + 1, elements);
        doc.Elements.AddRange(appended);

        return insertions.Count + appendedImages;
    }

    /// <summary>
    /// The elements one recognition contributes.
    /// </summary>
    /// <remarks>
    /// A recognizer reports a table region as HTML markup — see <see cref="OcrTables"/> — and
    /// markup in a text element is markup in every rendering of the document. Recovering it into
    /// the document's own table model here, while the element stream is still the only
    /// representation, is what lets Markdown write a pipe table, HTML write a real
    /// <c>&lt;table&gt;</c>, and plain text write neither's tags; it also puts the table in
    /// <c>ExtractedDocument.Tables</c>, where a consumer already looks for one.
    /// <para>
    /// What is left over is text, and markdown-shaped text is marked as such, because the markdown
    /// renderer has to know: text it treats as a paragraph's words gets escaped, which turns a
    /// recognised heading into <c>\## Heading</c>. Every other renderer ignores the marker and
    /// emits the text as it stands.
    /// </para>
    /// </remarks>
    private static List<InternalElement> ElementsFor(
        InternalDocument doc, string text, OcrElementLevel level, uint? page, OcrOptions options)
    {
        var elements = new List<InternalElement>();

        foreach (var segment in OcrTables.Split(text))
        {
            InternalElement element;
            if (segment.Cells is { } cells)
            {
                uint index = doc.PushTable(new Table
                {
                    Cells = cells,
                    Markdown = InternalDocumentBuilder.CellsToMarkdown(cells),
                    PageNumber = page ?? 0,
                });
                element = InternalElement.TextElement(ElementKind.Table(index), "", 0);
            }
            else
            {
                element = InternalElement.TextElement(ElementKind.OcrText(level), segment.Text, 0);

                if (options.TextFormat == OcrTextFormat.Markdown)
                {
                    element.Attributes ??= new Dictionary<string, string>();
                    element.Attributes[MarkdownAttribute] = "markdown";
                }
            }

            element.Page = page;
            elements.Add(element);
        }

        return elements;
    }

    /// <summary>
    /// Recognise one image, turning a per-image failure into a warning.
    /// </summary>
    /// <returns>
    /// False when the recognizer is gone for good and the caller should stop asking; true
    /// otherwise, including when this image simply yielded nothing.
    /// </returns>
    private static bool TryRecognize(
        InternalDocument doc, IOcrEngine engine, ReadOnlySpan<byte> bytes, OcrOptions options,
        string what, out OcrImageResult result)
    {
        result = OcrImageResult.Empty;
        // Linked to the call's own token: the per-image budget abandons one image, the caller's
        // cancellation abandons the whole extraction, and only the first becomes a warning.
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(Cancel.Token);
        timeout.CancelAfter(options.PerImageTimeout);
        try
        {
            result = engine.Recognize(bytes, timeout.Token);
            return true;
        }
        catch (OcrUnavailableException e)
        {
            Warn(doc, e.Message);
            return false;
        }
        catch (OperationCanceledException) when (Cancel.NotCancelling)
        {
            Warn(doc, $"OCR of {what} exceeded {options.PerImageTimeout.TotalSeconds:0.#}s and was abandoned");
            return true;
        }
        catch (Exception e) when (Cancel.NotCancelling)
        {
            Warn(doc, $"OCR of {what} failed: {e.Message}");
            return true;
        }
    }

    private static void Warn(InternalDocument doc, string message) =>
        doc.ProcessingWarnings.Add(new ProcessingWarning { Source = WarningSource, Message = message });
}
