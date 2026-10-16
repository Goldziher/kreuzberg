namespace XRay.Content.Internal.Pdf;

/// <summary>Dominant raster codec on a page.</summary>
internal enum ImageCodecClass
{
    /// <summary>No raster images.</summary>
    None,
    /// <summary>CCITT Group 3/4 fax — 1-bit, almost always a scan.</summary>
    Ccitt,
    /// <summary>JBIG2 — 1-bit, almost always a scan.</summary>
    Jbig2,
    /// <summary>DCT/JPEG — photo or colour scan.</summary>
    Dct,
    /// <summary>Flate/other raster.</summary>
    Other,
}

/// <summary>Document-level scanner-vs-authoring prior. Weak, never decisive.</summary>
internal enum ProducerPrior { Scanner, Authoring, Unknown }

/// <summary>Per-page evidence, gathered without decoding image pixels.</summary>
internal sealed class PageScanSignals
{
    /// <summary>Fraction of the page covered by raster images, clamped to [0, 1].</summary>
    public double ImageCoverage;
    /// <summary>Fraction of shown text drawn invisibly (text render mode 3), in [0, 1].</summary>
    public double InvisibleTextRatio;
    /// <summary>Number of glyphs in the native text layer.</summary>
    public int GlyphCount;
    public ImageCodecClass Codec;
    public ProducerPrior ProducerPrior;
}

/// <summary>Document-level detection outcome.</summary>
internal sealed class ScanDetection
{
    /// <summary>Highest per-page confidence in the document, in [0, 1].</summary>
    public double Confidence;
    /// <summary>Per-page confidence, indexed by zero-based page number.</summary>
    public List<double> PageConfidence = new();

    /// <summary>One-based page numbers scoring at or above <paramref name="minConfidence"/>.</summary>
    public List<uint> ScannedPageNumbers(double minConfidence)
    {
        double threshold = Math.Clamp(minConfidence, 0.0, 1.0);
        var pages = new List<uint>();
        for (int i = 0; i < PageConfidence.Count; i++)
            if (PageConfidence[i] >= threshold) pages.Add((uint)i + 1);
        return pages;
    }
}

/// <summary>
/// Scanned-page detection for PDFs. Ports Rust <c>crates/xberg/src/pdf/scan_detect.rs</c>
/// together with the per-page signal gathering pdf_oxide performs for it.
/// <para>
/// Advisory only: a page that cannot be graded scores 0.0 rather than failing extraction.
/// </para>
/// </summary>
internal static class PdfScanDetect
{
    /// <summary>Below this raster coverage a page is text with a figure, never a scan.</summary>
    private const double ImageCoverageMin = 0.80;

    /// <summary>Fraction of glyphs in render mode 3 (invisible) that marks an OCR sidecar.</summary>
    private const double InvisibleTextMin = 0.50;

    /// <summary>A full-page raster alone. Below every usable threshold: a slide with a
    /// full-bleed background image scores exactly this.</summary>
    private const double ScoreFullPageRaster = 0.50;

    /// <summary>Added when the text layer is hidden or absent.</summary>
    private const double ScoreNoVisibleText = 0.35;

    /// <summary>Added for CCITT/JBIG2: bilevel fax codecs, not emitted by authoring tools.</summary>
    private const double ScoreBilevelCodec = 0.10;

    /// <summary>Added when the producer names scanner software. A weak prior, never decisive.</summary>
    private const double ScoreScannerProducer = 0.05;

    /// <summary>Default `scanned_min_confidence` from the Rust OCR config.</summary>
    public const double DefaultScannedMinConfidence = 0.70;

    private static readonly string[] ScannerKeywords =
        { "scan", "abbyy", "tesseract", "scansnap", "finereader", "ocr", "lens", "camscanner", "kofax" };

    private static readonly string[] AuthoringKeywords =
    {
        "word", "libreoffice", "latex", "pdftex", "chromium", "skia", "quartz", "wkhtmltopdf",
        "pdf_oxide", "reportlab", "prince", "weasyprint", "powerpoint", "excel", "indesign",
    };

    /// <summary>Grade one page's evidence. Pure, so it is testable without a document.</summary>
    /// <remarks>
    /// The terms accumulate in single precision because the reference does, and the sum is
    /// reported verbatim: 0.50 + 0.35 + 0.05 lands on 0.90000004 in float and 0.9 in double, and
    /// the two serialize to visibly different numbers.
    /// </remarks>
    public static double ScorePage(PageScanSignals s)
    {
        if (s.ImageCoverage < ImageCoverageMin) return 0.0;

        float score = (float)ScoreFullPageRaster;

        if (s.GlyphCount == 0 || s.InvisibleTextRatio >= InvisibleTextMin)
            score += (float)ScoreNoVisibleText;

        if (s.Codec is ImageCodecClass.Ccitt or ImageCodecClass.Jbig2)
            score += (float)ScoreBilevelCodec;

        if (s.ProducerPrior == ProducerPrior.Scanner)
            score += (float)ScoreScannerProducer;

        return Math.Clamp(score, 0f, 1f);
    }

    /// <summary>
    /// Grade every page of <paramref name="doc"/>. Infallible: an unreadable page scores 0.0.
    /// </summary>
    public static ScanDetection Detect(PdfDocument doc)
    {
        var prior = ClassifyProducer(doc);
        var detection = new ScanDetection();

        int pageCount;
        try { pageCount = doc.PageCount; }
        catch (Exception) when (Cancel.NotCancelling) { return detection; }

        for (int i = 0; i < pageCount; i++)
        {
            Cancel.ThrowIfRequested();
            double score;
            try
            {
                var signals = PageSignals(doc, i, prior);
                score = signals is null ? 0.0 : ScorePage(signals);
            }
            catch (Exception) when (Cancel.NotCancelling) { score = 0.0; }
            detection.PageConfidence.Add(score);
        }

        foreach (double s in detection.PageConfidence)
            if (s > detection.Confidence) detection.Confidence = s;

        return detection;
    }

    /// <summary>
    /// Signals for one page, or <c>null</c> when it yields no evidence. Pages under
    /// <see cref="ImageCoverageMin"/> skip the content-stream inspection: it cannot lift
    /// their score above zero.
    /// </summary>
    private static PageScanSignals? PageSignals(PdfDocument doc, int pageIndex, ProducerPrior prior)
    {
        var (llx, lly, urx, ury) = doc.GetPageMediaBox(pageIndex);
        double left = Math.Min(llx, urx), right = Math.Max(llx, urx);
        double bottom = Math.Min(lly, ury), top = Math.Max(lly, ury);
        double pageArea = (right - left) * (top - bottom);
        if (pageArea <= double.Epsilon) return null;

        var walk = PdfContentScan.Walk(doc, pageIndex);

        // Overlapping images are summed, not unioned, so this is an upper bound: it may
        // over-select a page for inspection, never under-select one.
        double covered = 0;
        foreach (var (x0, y0, x1, y1) in walk.ImageBoxes)
        {
            double w = Math.Min(x1, right) - Math.Max(x0, left);
            double h = Math.Min(y1, top) - Math.Max(y0, bottom);
            covered += Math.Max(w, 0) * Math.Max(h, 0);
        }
        double coverage = Math.Clamp(covered / pageArea, 0.0, 1.0);
        if (coverage < ImageCoverageMin) return null;

        return new PageScanSignals
        {
            ImageCoverage = coverage,
            InvisibleTextRatio = walk.TextBytes == 0 ? 0.0 : (double)walk.InvisibleBytes / walk.TextBytes,
            GlyphCount = walk.TextBytes,
            Codec = walk.Codec,
            ProducerPrior = prior,
        };
    }

    private static ProducerPrior ClassifyProducer(PdfDocument doc)
    {
        string producer = InfoString(doc, "Producer");
        string creator = InfoString(doc, "Creator");
        string p = (producer + " " + creator).ToLowerInvariant();

        foreach (var k in ScannerKeywords)
            if (p.Contains(k, StringComparison.Ordinal)) return ProducerPrior.Scanner;
        foreach (var k in AuthoringKeywords)
            if (p.Contains(k, StringComparison.Ordinal)) return ProducerPrior.Authoring;
        return ProducerPrior.Unknown;
    }

    private static string InfoString(PdfDocument doc, string key) => doc.Resolve(doc.InfoDict?.Get(key)) switch
    {
        PdfString s => PdfMetadataExtractor.DecodePdfString(s.Bytes) ?? "",
        PdfName n => n.Value,
        _ => "",
    };
}
