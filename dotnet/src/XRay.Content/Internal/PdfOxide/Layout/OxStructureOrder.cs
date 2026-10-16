// Ported from pdf_oxide `pipeline/reading_order/structure_tree.rs`
// (`StructureTreeStrategy::apply`, `mcid_order_zigzags_columns`, l. 1-205) and the
// `/MarkInfo` half of `document.rs::struct_tree_trustworthy` (l. 3828-3855).
//
// This is tier 1 of `pipeline::page_reading_order`: a tagged document's `/StructTreeRoot`
// records the producer's own logical order (§14.7.1), which is authoritative over glyph
// geometry. `TextPipelineConfig::default()` selects `StructureTreeFirst`, so every caller
// that reads its spans through `page_reading_order` — the word path, and so the spatial
// table detector — sees structure order on a tagged, non-suspect page.
//
// The strategy's own fallbacks (article threads, then XY-cut) collapse to the XY-cut here:
// `page_article_bead_rects` is unported and fails closed, which is the position the Rust
// reaches on every document without `/Threads`.
using System;
using System.Collections.Generic;
using XRay.Content.Internal.Pdf;
using XRay.Content.Internal.PdfOxide.Structure;
using XRay.Content.Internal.PdfOxide.Text;

namespace XRay.Content.Internal.PdfOxide.Layout;

internal static class OxStructureOrder
{
    /// <summary>
    /// The page's `(scope, mcid)` pre-order projected to bare MCIDs, or null when the
    /// structure tree is not trustworthy for ordering (`build_context`).
    /// </summary>
    /// <remarks>
    /// `/MarkInfo /Suspects true` is the spec's signal (§14.8.2.3.1) that page content
    /// order may not match logical structure order, so the tree is rejected for ordering
    /// and the geometric tier takes over. The /ActualText index deliberately does NOT
    /// share this gate — see <see cref="OxActualText"/> — so the check lives here.
    /// </remarks>
    public static List<int>? McidOrderForPage(PdfDocument doc, int pageIndex)
    {
        if (Suspects(doc)) return null;
        var scoped = OxActualText.McidOrderForPage(doc, pageIndex);
        if (scoped.Count == 0) return null;
        var order = new List<int>(scoped.Count);
        foreach (var (_, mcid) in scoped) order.Add(mcid);
        return order;
    }

    /// <summary>`/MarkInfo /Suspects` (§14.7.1), false when absent or unreadable.</summary>
    private static bool Suspects(PdfDocument doc)
    {
        try
        {
            if (doc.Catalog?.Get("MarkInfo") is not { } raw) return false;
            return doc.Resolve(raw).AsDict()?.Get("Suspects").AsBool() ?? false;
        }
        catch (Exception) when (Cancel.NotCancelling) { return false; }
    }

    /// <summary>
    /// Order spans by the structure tree's MCID sequence, appending the spans it does not
    /// cover in geometric order (`StructureTreeStrategy::apply`).
    /// </summary>
    /// <remarks>
    /// Falls back wholesale to the XY-cut when the MCID sequence would zigzag across a
    /// clear multi-column layout — the signature of a producer that numbered marked
    /// content in content-stream order rather than reading order.
    /// </remarks>
    public static List<OxTextSpan> Apply(List<OxTextSpan> spans, IReadOnlyList<int> mcidOrder)
    {
        if (mcidOrder.Count == 0 || McidOrderZigzagsColumns(spans, mcidOrder))
            return OxReadingOrder.OrderSpansColumnAware(spans);

        // Last occurrence wins, as the Rust `HashMap` collect does for a repeated MCID.
        var mcidToOrder = new Dictionary<int, int>(mcidOrder.Count);
        for (int i = 0; i < mcidOrder.Count; i++) mcidToOrder[mcidOrder[i]] = i;

        var withMcid = new List<(OxTextSpan Span, int Order)>(spans.Count);
        var withoutMcid = new List<OxTextSpan>();
        foreach (var span in spans)
        {
            if (span.Mcid is { } m && mcidToOrder.TryGetValue(m, out int order))
                withMcid.Add((span, order));
            else
                withoutMcid.Add(span);
        }

        // Stable, so spans sharing one MCID keep the order they arrived in.
        OxSpanCompare.SortStable(withMcid, static (a, b) => a.Order.CompareTo(b.Order));

        var result = new List<OxTextSpan>(spans.Count);
        foreach (var (span, _) in withMcid) result.Add(span);
        if (withoutMcid.Count > 0) result.AddRange(OxReadingOrder.OrderSpansColumnAware(withoutMcid));
        return result;
    }

    /// <summary>
    /// Whether applying <paramref name="mcidOrder"/> would zigzag horizontally across a
    /// two-column layout (`mcid_order_zigzags_columns`).
    /// </summary>
    internal static bool McidOrderZigzagsColumns(
        IReadOnlyList<OxTextSpan> spans, IReadOnlyList<int> mcidOrder)
    {
        // Last span wins for a repeated MCID, matching the Rust `HashMap` collect.
        var mcidToIdx = new Dictionary<int, int>();
        for (int i = 0; i < spans.Count; i++)
            if (spans[i].Mcid is { } m) mcidToIdx[m] = i;

        var orderedX = new List<float>(mcidOrder.Count);
        foreach (int m in mcidOrder)
            if (mcidToIdx.TryGetValue(m, out int i))
                orderedX.Add(spans[i].Bbox.X + spans[i].Bbox.Width * 0.5f);
        if (orderedX.Count < 10) return false;

        var sorted = new List<float>(orderedX);
        OxSpanCompare.SortStable(sorted, static (a, b) => OxSpanCompare.SafeFloatCmp(a, b));
        float xMin = sorted[0], xMax = sorted[^1];
        float extent = xMax - xMin;
        if (extent < 50.0f) return false; // single column

        float largestGap = 0.0f, largestGapAt = xMin;
        for (int i = 1; i < sorted.Count; i++)
        {
            float gap = sorted[i] - sorted[i - 1];
            if (gap > largestGap)
            {
                largestGap = gap;
                largestGapAt = (sorted[i - 1] + sorted[i]) * 0.5f;
            }
        }
        // A gutter is a substantial fraction of the text extent, not inter-word whitespace.
        if (largestGap < extent * 0.1f || largestGap < 30.0f) return false;

        int crossings = 0;
        for (int i = 1; i < orderedX.Count; i++)
            if ((orderedX[i - 1] < largestGapAt) != (orderedX[i] < largestGapAt))
                crossings++;
        // A column-respecting order crosses once, at the bottom of one column into the top
        // of the next; more than three crossings is interleaving.
        return crossings > 3;
    }
}
