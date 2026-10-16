// Page annotations, ported from crates/xberg/src/pdf/native/annotations.rs.
//
// Every non-widget, non-popup annotation of every page becomes a PdfAnnotation: its subtype,
// the comment (or a link's URI), /Rect, author, dates, colour, subject, and for the four
// text-markup types the /QuadPoints boxes and the page text underneath them. Form widgets are
// the form-field reader's business (PdfFormFields) and popups only position a parent's text.
using System.Globalization;
using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Internal.Pdf;

internal static class PdfAnnotations
{
    /// <summary>A hostile /Annots array cannot make one page cost more than this many entries.</summary>
    private const int MaxAnnotationsPerPage = 20_000;
    private const int MaxAnnotationsTotal = 200_000;

    /// <summary>
    /// Read the annotations of every page. Pages whose /Annots cannot be read contribute a
    /// warning (source <c>pdf_annotations</c>) rather than failing the document.
    /// </summary>
    /// <param name="pdf">The document.</param>
    /// <param name="pageWords">
    /// The page text as words with geometry (index = page index), for marked-text recovery.
    /// Null skips that step: markup annotations still carry their quad boxes.
    /// </param>
    /// <param name="warnings">Where a per-page failure is reported.</param>
    public static List<PdfAnnotation> Extract(
        PdfDocument pdf, List<List<TableSpan>>? pageWords, List<ProcessingWarning> warnings)
    {
        var result = new List<PdfAnnotation>();
        int pageCount;
        try { pageCount = pdf.PageCount; }
        catch (Exception e) when (Cancel.NotCancelling)
        {
            warnings.Add(new ProcessingWarning
            {
                Source = "pdf_annotations",
                Message = $"annotation extraction failed: could not determine page count ({e.Message}); no annotations were extracted",
            });
            return result;
        }

        var links = PdfLinkResolver.For(pdf);
        for (int pageIndex = 0; pageIndex < pageCount; pageIndex++)
        {
            Cancel.ThrowIfRequested();
            try
            {
                ExtractPage(pdf, pageIndex, pageWords, links, result);
            }
            catch (Exception e) when (Cancel.NotCancelling)
            {
                warnings.Add(new ProcessingWarning
                {
                    Source = "pdf_annotations",
                    Message = $"annotation extraction failed for page {pageIndex + 1}: {e.Message}; annotations on this page were skipped",
                });
            }
            if (result.Count >= MaxAnnotationsTotal) break;
        }
        return result;
    }

    /// <summary>The annotation dictionaries of a page, in order, unresolved entries dropped.</summary>
    public static IEnumerable<PdfDict> PageAnnotationDicts(PdfDocument pdf, int pageIndex)
    {
        var annots = pdf.Resolve(pdf.Pages[pageIndex].Get("Annots")).AsArray();
        if (annots is null) yield break;
        int n = 0;
        foreach (var item in annots.Items)
        {
            if (n++ >= MaxAnnotationsPerPage) yield break;
            Cancel.Every(n, 0xFF);
            var dict = pdf.Resolve(item).AsDict();
            if (dict is not null) yield return dict;
        }
    }

    private static void ExtractPage(
        PdfDocument pdf, int pageIndex, List<List<TableSpan>>? pageWords, PdfLinkResolver links,
        List<PdfAnnotation> result)
    {
        var (pageW, pageH) = PageSize(pdf, pageIndex);
        foreach (var dict in PageAnnotationDicts(pdf, pageIndex))
        {
            string? subtype = pdf.Resolve(dict.Get("Subtype")).AsName();
            if (subtype is "Widget" or "Popup") continue;

            var type = MapSubtype(subtype);
            var annotation = new PdfAnnotation
            {
                AnnotationType = type,
                Subtype = subtype,
                PageNumber = (uint)(pageIndex + 1),
                PageWidth = pageW,
                PageHeight = pageH,
                Flags = pdf.Resolve(dict.Get("F")).AsLong() is { } f ? (uint)f : null,
            };

            string? contents = Text(pdf, dict, "Contents");
            annotation.Author = NonEmpty(Text(pdf, dict, "T"));
            annotation.Modified = NonEmpty(Text(pdf, dict, "M"));
            annotation.Created = NonEmpty(Text(pdf, dict, "CreationDate"));
            annotation.Subject = NonEmpty(Text(pdf, dict, "Subj"));
            annotation.Color = ColorToHex(Numbers(pdf, dict.Get("C")));
            annotation.BoundingBox = Rect(Numbers(pdf, dict.Get("Rect")));

            // A link's content is its URI; an internal one is described by Destination (and the
            // anchor URI the extractor adds), since it has no text of its own.
            if (type == PdfAnnotationType.Link)
            {
                var action = pdf.Resolve(dict.Get("A")).AsDict();
                string? uri = action is not null && pdf.Resolve(action.Get("S")).AsName() == "URI"
                    ? links.UriText(action)
                    : null;
                if (!string.IsNullOrEmpty(uri)) annotation.Content = uri;
                else
                {
                    var internalLink = links.DestinationUri(dict.Get("Dest"), contents)
                                       ?? links.ActionUri(action, contents);
                    annotation.Destination = internalLink?.Url;
                }
            }
            if (annotation.Content is null)
            {
                // Free-text boxes often keep their words only in the rich-text string.
                contents = NonEmpty(contents) ?? NonEmpty(StripMarkup(Text(pdf, dict, "RC")));
                annotation.Content = contents;
            }

            if (IsTextMarkup(type))
            {
                var quads = QuadBoxes(Numbers(pdf, dict.Get("QuadPoints")));
                if (quads.Count > 0)
                {
                    annotation.QuadPoints = quads;
                    if (pageWords is not null && pageIndex < pageWords.Count)
                        annotation.MarkedText = MarkedText(pageWords[pageIndex], quads);
                }
            }
            result.Add(annotation);
        }
    }

    /// <summary>The visible box (CropBox, else MediaBox) of a page.</summary>
    public static (double Width, double Height) PageSize(PdfDocument pdf, int pageIndex)
    {
        var crop = Numbers(pdf, pdf.Pages[pageIndex].Get("CropBox"));
        if (crop is { Count: >= 4 }) return (Math.Abs(crop[2] - crop[0]), Math.Abs(crop[3] - crop[1]));
        var (llx, lly, urx, ury) = pdf.GetPageMediaBox(pageIndex);
        return (Math.Abs(urx - llx), Math.Abs(ury - lly));
    }

    internal static PdfAnnotationType MapSubtype(string? subtype) => subtype switch
    {
        "Text" or "FreeText" => PdfAnnotationType.Text,
        "Highlight" => PdfAnnotationType.Highlight,
        "Link" => PdfAnnotationType.Link,
        "Stamp" => PdfAnnotationType.Stamp,
        "Underline" => PdfAnnotationType.Underline,
        "StrikeOut" => PdfAnnotationType.StrikeOut,
        "Squiggly" => PdfAnnotationType.Squiggly,
        "Ink" => PdfAnnotationType.Ink,
        "Square" => PdfAnnotationType.Square,
        "Circle" => PdfAnnotationType.Circle,
        "Polygon" => PdfAnnotationType.Polygon,
        "PolyLine" => PdfAnnotationType.PolyLine,
        "Line" => PdfAnnotationType.Line,
        "Caret" => PdfAnnotationType.Caret,
        "FileAttachment" => PdfAnnotationType.FileAttachment,
        "Sound" => PdfAnnotationType.Sound,
        "Movie" => PdfAnnotationType.Movie,
        _ => PdfAnnotationType.Other,
    };

    private static bool IsTextMarkup(PdfAnnotationType type) =>
        type is PdfAnnotationType.Highlight or PdfAnnotationType.Underline
            or PdfAnnotationType.StrikeOut or PdfAnnotationType.Squiggly;

    private static string? Text(PdfDocument pdf, PdfDict dict, string key) =>
        pdf.Resolve(dict.Get(key)) switch
        {
            PdfString s => PdfFormFields.DecodeTextString(s.Bytes),
            PdfStream st => PdfFormFields.DecodeTextString(pdf.DecodeStream(st)),
            _ => null,
        };

    private static string? NonEmpty(string? s) => string.IsNullOrEmpty(s) ? null : s;

    /// <summary>The words of a rich-text (XHTML) string, without its tags.</summary>
    private static string? StripMarkup(string? rich)
    {
        if (string.IsNullOrEmpty(rich)) return null;
        var sb = new StringBuilder(rich.Length);
        bool inTag = false;
        foreach (char c in rich)
        {
            if (c == '<') { inTag = true; continue; }
            if (c == '>') { inTag = false; sb.Append(' '); continue; }
            if (!inTag) sb.Append(c);
        }
        string text = System.Net.WebUtility.HtmlDecode(sb.ToString());
        return string.Join(' ', text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries));
    }

    private static List<double>? Numbers(PdfDocument pdf, PdfObject? array)
    {
        if (pdf.Resolve(array).AsArray() is not { } a) return null;
        var list = new List<double>(a.Items.Count);
        foreach (var item in a.Items)
            if (pdf.Resolve(item).AsNumber() is { } n) list.Add(n);
        return list;
    }

    private static BoundingBox? Rect(List<double>? r) =>
        r is { Count: >= 4 }
            ? new BoundingBox { X0 = Math.Min(r[0], r[2]), Y0 = Math.Min(r[1], r[3]), X1 = Math.Max(r[0], r[2]), Y1 = Math.Max(r[1], r[3]) }
            : null;

    /// <summary>
    /// Reduce each <c>/QuadPoints</c> quad to its axis-aligned box. The spec does not fix the
    /// winding order of the four points, so the min and max of each axis are taken.
    /// </summary>
    internal static List<BoundingBox> QuadBoxes(List<double>? q)
    {
        var boxes = new List<BoundingBox>();
        if (q is null) return boxes;
        for (int i = 0; i + 7 < q.Count && boxes.Count < 10_000; i += 8)
        {
            double x0 = Math.Min(Math.Min(q[i], q[i + 2]), Math.Min(q[i + 4], q[i + 6]));
            double x1 = Math.Max(Math.Max(q[i], q[i + 2]), Math.Max(q[i + 4], q[i + 6]));
            double y0 = Math.Min(Math.Min(q[i + 1], q[i + 3]), Math.Min(q[i + 5], q[i + 7]));
            double y1 = Math.Max(Math.Max(q[i + 1], q[i + 3]), Math.Max(q[i + 5], q[i + 7]));
            boxes.Add(new BoundingBox { X0 = x0, Y0 = y0, X1 = x1, Y1 = y1 });
        }
        return boxes;
    }

    /// <summary>
    /// The page text under each quad box, one piece per box joined by a space. A word counts as
    /// covered when its centre is inside the box or at least half its area is.
    /// </summary>
    internal static string? MarkedText(List<TableSpan> words, List<BoundingBox> quads)
    {
        var pieces = new List<string>();
        foreach (var quad in quads)
        {
            var hit = new List<(double X, string Text)>();
            foreach (var w in words)
            {
                if (w.Text.Length == 0) continue;
                if (!Covers(quad, w.Bbox)) continue;
                hit.Add((w.Bbox.Left, w.Text));
            }
            if (hit.Count == 0) continue;
            // Words arrive in reading order already; a stable sort by x keeps a line left to right
            // without reshuffling words that share a position.
            string piece = string.Join(' ', hit.OrderBy(h => h.X).Select(h => h.Text)).Trim();
            if (piece.Length > 0) pieces.Add(piece);
        }
        return pieces.Count == 0 ? null : string.Join(' ', pieces);
    }

    private static bool Covers(BoundingBox quad, PathRect word)
    {
        // A word's box is stored with its extents either way up; take the span of each axis.
        double wx0 = word.Left, wx1 = word.Right;
        double wy0 = Math.Min(word.Top, word.Bottom), wy1 = Math.Max(word.Top, word.Bottom);
        double cx = (wx0 + wx1) / 2, cy = (wy0 + wy1) / 2;
        if (cx >= quad.X0 && cx <= quad.X1 && cy >= quad.Y0 && cy <= quad.Y1) return true;
        double ix = Math.Min(wx1, quad.X1) - Math.Max(wx0, quad.X0);
        double iy = Math.Min(wy1, quad.Y1) - Math.Max(wy0, quad.Y0);
        double area = (wx1 - wx0) * (wy1 - wy0);
        return ix > 0 && iy > 0 && area > 0 && ix * iy / area >= 0.5;
    }

    /// <summary>A <c>/C</c> array (gray, RGB or CMYK) as <c>#rrggbb</c>.</summary>
    internal static string? ColorToHex(IReadOnlyList<double>? c)
    {
        if (c is null) return null;
        static int U8(double v) => (int)Math.Round(Math.Clamp(v, 0.0, 1.0) * 255.0, MidpointRounding.AwayFromZero);
        switch (c.Count)
        {
            case 1:
                { int g = U8(c[0]); return Hex(g, g, g); }
            case 3:
                return Hex(U8(c[0]), U8(c[1]), U8(c[2]));
            case 4:
                return Hex(U8((1 - c[0]) * (1 - c[3])), U8((1 - c[1]) * (1 - c[3])), U8((1 - c[2]) * (1 - c[3])));
            default:
                return null;
        }
        static string Hex(int r, int g, int b) =>
            "#" + r.ToString("x2", CultureInfo.InvariantCulture) + g.ToString("x2", CultureInfo.InvariantCulture)
            + b.ToString("x2", CultureInfo.InvariantCulture);
    }
}
