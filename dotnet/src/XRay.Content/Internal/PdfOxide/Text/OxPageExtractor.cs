// Ported from pdf_oxide `document.rs`: `extract_spans_impl` (15460), `load_fonts` (19130),
// `page_cannot_have_text` (9629), `may_contain_text` (9578), `extract_chars_impl` (16508)
// and `extract_page_text_with_options` (15979).
//
// This is the page-level driver above the extractor: it decides whether a page can carry
// text at all, loads its fonts, runs the extractor over the content stream twice — once for
// spans, once for the glyph positions those spans are stamped with — then drops off-page
// spans and applies reading order.
//
// Font loading and the `BT`/`Do` pre-scan live on the extractor itself, ported alongside
// the operator dispatch that also needs them for Form XObjects.
using System;
using System.Collections.Generic;
using System.Linq;
using XRay.Content.Internal.Pdf;
using XRay.Content.Internal.PdfOxide.Fonts;
using XRay.Content.Internal.PdfOxide.Layout;
using XRay.Content.Internal.PdfOxide.Structure;

namespace XRay.Content.Internal.PdfOxide.Text;

internal static class OxPageExtractor
{
    /// <summary>
    /// One page seen by both span pipelines: the text path's spans and, separately, the
    /// word path's.
    /// </summary>
    /// <remarks>
    /// Upstream runs two pipelines over the same page and they diverge early. Text and
    /// hierarchy read `extract_spans_with_reading_order`, which drops off-page spans and
    /// sorts. Words read `extract_words` → `pipeline::page_reading_order` → `extract_spans`,
    /// whose `postprocess_spans` also merges drop caps, maps a /Rotate'd page's rotated
    /// content into the displayed frame, and rewrites text (super/subscripts, combining
    /// marks, typographic spaces). Both are served here from one content-stream pass so the
    /// second pipeline costs no extra parse.
    /// </remarks>
    internal sealed class OxPage
    {
        /// <summary>The text and hierarchy path's spans, plus the page dimensions.</summary>
        public OxPageText Text = new();

        /// <summary>The word path's spans, as `page_reading_order` leaves them.</summary>
        public List<OxTextSpan> WordSpans = new();

        /// <summary>
        /// The hierarchy path's spans, as `ReadingOrder::TopToBottom` leaves them.
        /// </summary>
        /// <remarks>
        /// The structure pipeline asks for TopToBottom where the text assembler asks for
        /// ColumnAware, and the two differ by more than the sort. The row-aware comparator
        /// bands near-equal Y values, so spans that tie on (band, x) keep the order they
        /// arrived in, and sorting the column-aware list would hand those ties a different
        /// starting sequence than sorting the raw one. These spans also carry no per-glyph
        /// x-origins, because the stamp that produces them belongs to `postprocess_spans`
        /// and the TopToBottom path never runs it.
        /// </remarks>
        public List<OxTextSpan> HierarchySpans = new();
    }

    /// <summary>
    /// One page's spans and glyphs, ordered as `ReadingOrder::ColumnAware` leaves them —
    /// the shape `extract_page_text_with_options` returns.
    /// </summary>
    public static OxPageText ExtractPageText(PdfDocument doc, int pageIndex) =>
        ExtractPage(doc, pageIndex).Text;

    /// <summary>Both span pipelines for one page, from a single content-stream pass.</summary>
    public static OxPage ExtractPage(PdfDocument doc, int pageIndex)
    {
        Cancel.ThrowIfRequested();
        var page = pageIndex >= 0 && pageIndex < doc.PageCount ? doc.Pages[pageIndex] : null;
        var (llx, lly, urx, ury) = doc.GetPageMediaBox(pageIndex);
        var result = new OxPage { Text = new OxPageText { PageWidth = (float)urx, PageHeight = (float)ury } };
        if (page is null) return result;

        var (spans, mcWins) = ExtractSpans(doc, pageIndex, page);
        if (spans.Count == 0) return result;

        var chars = ExtractChars(doc, pageIndex, page);

        // The word path works from its own copies: `postprocess_spans` rewrites text and
        // geometry in place, and the text path must not see either.
        var wordSpans = new List<OxTextSpan>(spans.Count);
        foreach (var s in spans) wordSpans.Add(s.Clone());
        wordSpans = OxSpanPostprocess.Run(doc, pageIndex, wordSpans, chars);
        result.WordSpans = OxPageOrder.PageReadingOrder(
            wordSpans,
            OxCharXOffsets.GetPageRotation(doc, pageIndex),
            ((float)llx, (float)lly, (float)urx, (float)ury),
            OxStructureOrder.McidOrderForPage(doc, pageIndex));

        // The hierarchy path takes its copies BEFORE the stamp below. `stamp_char_x_offsets`
        // lives inside `postprocess_spans`, which the `ReadingOrder::TopToBottom` path never
        // runs, so upstream's hierarchy spans carry no per-glyph origins at all and the
        // inline-script rejoin measures character positions from the advance widths instead.
        var hierarchySpans = new List<OxTextSpan>(spans.Count);
        foreach (var s in spans) hierarchySpans.Add(s.Clone());
        OxReadingOrder.DropOffpageSpans(hierarchySpans, (float)llx, (float)lly, (float)urx, (float)ury);
        OxSpanCompare.SortSpansRowAware(hierarchySpans);
        OxActualText.ApplyToSpans(doc, pageIndex, hierarchySpans, mcWins);
        result.HierarchySpans = hierarchySpans;

        // Stamp the char extractor's own per-glyph x-origins onto the finished spans, so
        // everything that decomposes a span through `to_chars` sees spec-aligned positions
        // rather than a prefix-sum of nominal widths. Runs last, on the post-processed
        // spans, so the alignment sees the same text the consumers do.
        OxCharXOffsets.Stamp(doc, pageIndex, spans, chars);

        OxReadingOrder.DropOffpageSpans(spans, (float)llx, (float)lly, (float)urx, (float)ury);
        result.Text.Spans = OxReadingOrder.OrderSpansColumnAware(spans);

        // Struct-tree-scope /ActualText (§14.9.4) is applied last, on the ordered list, the
        // way `extract_spans_filtered_with_reading_order` closes. The word path does not get
        // it: `postprocess_spans` never calls the applier.
        OxActualText.ApplyToSpans(doc, pageIndex, result.Text.Spans, mcWins);

        // `PageText.chars` is left empty: the per-glyph list is consumed by the stamp above
        // and nothing downstream asks for it whole.
        return result;
    }

    /// <summary>
    /// One page's glyphs, in the order `extract_chars_impl` (document.rs:16508) leaves them:
    /// top-to-bottom, then left-to-right.
    /// </summary>
    private static List<OxTextChar> ExtractChars(PdfDocument doc, int pageIndex, PdfDict page)
    {
        var prepared = NewPageExtractor(doc, pageIndex, page);
        if (prepared is null) return new List<OxTextChar>();

        List<OxTextChar> chars;
        try { chars = prepared.Value.Extractor.ExtractOwned(prepared.Value.Content); }
        catch (Exception) when (Cancel.NotCancelling) { return new List<OxTextChar>(); }

        OxSpanCompare.SortStable(chars, (a, b) =>
        {
            int yCmp = OxSpanCompare.SafeFloatCmp(b.Bbox.Y, a.Bbox.Y);
            return yCmp != 0 ? yCmp : OxSpanCompare.SafeFloatCmp(a.Bbox.X, b.Bbox.X);
        });
        return chars;
    }

    /// <summary>
    /// Raw spans for one page, before reading order, with the MCIDs whose in-stream
    /// `BDC /ActualText` the extractor already applied — `take_mc_actualtext_mcids`, which
    /// upstream stashes on the document for the struct-tree applier to defer to.
    /// </summary>
    private static (List<OxTextSpan> Spans, HashSet<int> McWins) ExtractSpans(
        PdfDocument doc, int pageIndex, PdfDict page)
    {
        var prepared = NewPageExtractor(doc, pageIndex, page);
        if (prepared is null) return (new List<OxTextSpan>(), new HashSet<int>());

        try
        {
            var spans = prepared.Value.Extractor.ExtractTextSpans(prepared.Value.Content);
            return (spans, prepared.Value.Extractor.TakeMcActualTextMcids());
        }
        catch (Exception) when (Cancel.NotCancelling) { return (new List<OxTextSpan>(), new HashSet<int>()); }
    }

    /// <summary>
    /// An extractor loaded with the page's fonts, resources and layer exclusions, plus the
    /// page's decoded content stream — everything both extraction passes need before they
    /// diverge. Null when the page cannot carry text at all.
    /// </summary>
    private static (OxTextExtractor Extractor, byte[] Content)? NewPageExtractor(
        PdfDocument doc, int pageIndex, PdfDict page)
    {
        if (PageCannotHaveText(doc, page)) return null;

        byte[] content;
        try { content = doc.GetPageContent(pageIndex); }
        catch (Exception) when (Cancel.NotCancelling) { return null; }
        if (content.Length == 0 || !OxTextExtractor.MayContainText(content)) return null;

        var extractor = new OxTextExtractor();
        extractor.SetPageIndex(pageIndex);
        extractor.SetExcludedLayers(PdfOptionalContent.DefaultOffOcgs(doc));

        var resources = page.Get("Resources");
        if (resources is not null)
        {
            extractor.SetResources(resources);
            extractor.SetDocument(doc);
            extractor.LoadFontsForResources(extractor, resources);
        }

        return (extractor, content);
    }

    /// <summary>
    /// Whether the page definitely cannot produce text, so its content stream need not be
    /// decompressed at all. Conservative: anything unreadable counts as "might have text".
    /// </summary>
    private static bool PageCannotHaveText(PdfDocument doc, PdfDict page)
    {
        if (page.Get("Resources") is not { } resourcesObj) return true;
        var res = Ox.Dict(doc, resourcesObj);
        if (res is null) return false;

        if (Ox.GetDict(doc, res, "Font") is { } fonts && fonts.Map.Count > 0) return false;

        // A Form XObject can carry text of its own, so its presence rules nothing out.
        if (Ox.GetDict(doc, res, "XObject") is { } xobjects)
            foreach (var entry in xobjects.Map.Values)
            {
                var sub = doc.Resolve(entry);
                if (sub is PdfStream st && Ox.GetName(doc, st.Dict, "Subtype") == "Form") return false;
                if (sub.AsDict() is { } d && Ox.GetName(doc, d, "Subtype") == "Form") return false;
            }

        return true;
    }

}
