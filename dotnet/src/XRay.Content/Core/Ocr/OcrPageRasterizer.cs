using PaddleOcrSharp.Imaging;
using PaddleOcrSharp.Pdf;

namespace XRay.Content.Core.Ocr;

/// <summary>
/// Renders selected PDF pages to images, so a scanned page has something to recognise.
/// </summary>
/// <remarks>
/// <para>
/// This is the one part of OCR that needs a second native dependency: PDFium, through
/// <c>PaddleOCR.Pdf</c>. The port's own PDF reader extracts text and geometry but does not
/// rasterise, and a scanned page's text exists only as pixels — so without a rasteriser
/// <see cref="OcrMode.ScanOnly"/> could not work at all. See "Deviation: optional OCR" in
/// <c>CLAUDE.md</c> for why that trade-off was taken.
/// </para>
/// <para>
/// Pages come back PNG-encoded rather than as the library's own image type, which costs an
/// encode here and a decode in the recognizer. That is deliberate: it keeps
/// <see cref="IOcrEngine"/> free of PaddleOCR types, so the surrounding flow stays testable
/// without the package's models. Recognition costs orders of magnitude more than the round trip.
/// </para>
/// </remarks>
internal static class OcrPageRasterizer
{
    /// <summary>
    /// Render the requested pages, in page order.
    /// </summary>
    /// <param name="pdf">The whole PDF.</param>
    /// <param name="pages">One-based page numbers to render.</param>
    /// <param name="dpi">Rendering resolution.</param>
    /// <remarks>
    /// The underlying renderer only walks a document from its first page, so rendering stops at
    /// the highest page asked for and pages in between are rendered and discarded. For a wholly
    /// scanned document — the case this mode exists for — every page is wanted and nothing is
    /// wasted; for a mostly-native document with one scanned page near the end, the pages before
    /// it are rendered for nothing. Rendering a chosen page directly would mean depending on
    /// PDFium's wrapper package as well, which is a larger dependency change than the saving is
    /// worth.
    /// </remarks>
    public static List<(uint Page, byte[] Png)> Render(byte[] pdf, uint[] pages, int dpi)
    {
        var wanted = new HashSet<uint>(pages);
        if (wanted.Count == 0) return new List<(uint, byte[])>();

        int highest = (int)pages.Max();
        var rendered = new List<(uint Page, byte[] Png)>(wanted.Count);

        uint pageNumber = 0;
        foreach (var image in PdfRasterizer.Render(pdf, dpi, maxPages: highest))
        {
            Cancel.ThrowIfRequested();
            pageNumber++;
            using (image)
            {
                if (wanted.Contains(pageNumber))
                    rendered.Add((pageNumber, ImageIO.EncodePng(image)));
            }
            if (pageNumber >= highest) break;
        }

        return rendered;
    }
}
