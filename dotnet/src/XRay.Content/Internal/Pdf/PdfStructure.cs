// PDF structure / heading pipeline (oxide heuristic path).
//
// Port of the non-layout, non-structure-tree branch of
//   crates/xberg/src/pdf/structure/pipeline.rs :: extract_document_structure_from_segments
// plus the helpers it calls in hierarchy/clustering.rs, structure/classify.rs,
// structure/paragraphs.rs, structure/assembly.rs, structure/text_repair.rs,
// structure/lines.rs, structure/layout_classify.rs, structure/regions/heading.rs.
//
// Consumes per-page font-metric segments (ColumnAware reading order) and produces a
// structured InternalDocument with Heading/Paragraph/ListItem/Code elements. This is the
// document the Markdown/Djot/HTML renderers consume, giving heading-aware output that
// matches the Rust golden. Tables/images are out of scope here (Lever 1).
using System.Text;
using System.Text.RegularExpressions;
using XRay.Content.Types;

namespace XRay.Content.Internal.Pdf;

/// <summary>Font-metric text segment (mirrors Rust `SegmentData`).</summary>
public sealed class SegmentData
{
    public string Text = "";
    public float X, Y, Width, Height;
    public float FontSize;
    public bool IsBold;
    public bool IsItalic;
    public bool IsMonospace;
    public float BaselineY;

    /// <summary>
    /// Heading level (1-6) the PDF structure tree assigned, or <c>null</c> when it is unknown and
    /// must be inferred from font-size clustering.
    /// </summary>
    public byte? AssignedRole;

    /// <summary>
    /// Text-matrix rotation the span producer reported, in degrees.
    /// </summary>
    /// <remarks>
    /// The producer's page-space geometry stays in <see cref="X"/>, <see cref="Y"/>,
    /// <see cref="Width"/> and <see cref="Height"/>, which is what layout and table projection
    /// want. Reading-order and spacing arithmetic must not use it directly: a rotated run's
    /// width and height are flattened onto the run's own axis, not the page's, so an upright
    /// gap or baseline comparison across one is meaningless. Use the helpers below.
    /// </remarks>
    public float RotationDegrees;

    /// <summary>Rust <c>f32::EPSILON</c>: machine epsilon, not C#'s smallest denormal.</summary>
    private const float F32Epsilon = 1.1920929e-7f;

    /// <summary>Painted on the page's upright text axis.</summary>
    public bool IsUnrotated => MathF.Abs(RotationDegrees) <= F32Epsilon;

    /// <summary>Two segments share a reading frame, so their geometry is comparable.</summary>
    public bool HasSameRotation(SegmentData other) =>
        MathF.Abs(RotationDegrees - other.RotationDegrees) <= F32Epsilon;

    /// <summary>Page-space origin turned into this segment's own upright reading frame:
    /// (advance along its baseline, cross axis its visual lines stack on). The identity for an
    /// unrotated segment.</summary>
    public (float Advance, float Cross) UprightOrigin()
    {
        if (IsUnrotated) return (X, Y);
        float radians = -RotationDegrees * MathF.PI / 180.0f;
        float sin = MathF.Sin(radians), cos = MathF.Cos(radians);
        return (X * cos - Y * sin, X * sin + Y * cos);
    }

    /// <summary>(start, end) along this segment's reading direction.</summary>
    public (float Start, float End) UprightAdvanceExtent()
    {
        float start = UprightOrigin().Advance;
        return (start, start + Width);
    }

    /// <summary>(low, high) along the axis its visual lines stack on.</summary>
    public (float Low, float High) UprightCrossExtent()
    {
        float low = UprightOrigin().Cross;
        return (low, low + Height);
    }

    /// <summary>Baseline coordinate in this segment's own upright frame.</summary>
    public float UprightBaseline() => IsUnrotated ? BaselineY : UprightOrigin().Cross;

    public SegmentData Clone() => (SegmentData)MemberwiseClone();
}

internal sealed class PdfLine
{
    public List<SegmentData> Segments = new();
    public float BaselineY;
    public float DominantFontSize;
    public bool IsBold;
    public bool IsMonospace;
}

internal sealed class PdfParagraph
{
    public string Text = "";
    public List<PdfLine> Lines = new();
    public float DominantFontSize;
    public byte? HeadingLevel;
    public bool IsBold;
    public bool IsListItem;
    public bool IsCodeBlock;
    public bool IsFormula;
    public bool IsPageFurniture;
    // (left, bottom, right, top) PDF coords.
    public (float L, float B, float R, float T)? BlockBbox;
    public int WordCount;
}

public static class PdfStructure
{
    // constants.rs
    private const int MAX_HEADING_WORD_COUNT = 20;
    private const float MAX_HEADING_DISTANCE_MULTIPLIER = 2.0f;
    private const float MIN_HEADING_FONT_RATIO = 1.15f;
    private const float MIN_HEADING_FONT_GAP = 1.5f;

    /// <summary>A document with fewer blocks than this has too little evidence for the
    /// first-paragraph H1 rescue: its one paragraph is not "the biggest text on a page of
    /// body copy", it is the whole document (Rust `MIN_BLOCKS_FOR_FONT_HEADING`).</summary>
    private const int MIN_BLOCKS_FOR_FONT_HEADING = 5;

    /// <summary>Pages that must share the enlarged opening tier before a sparse document is
    /// allowed a heading level at all (Rust `SPARSE_REPEATED_TIER_MIN_PAGES`).</summary>
    private const int SPARSE_REPEATED_TIER_MIN_PAGES = 2;
    /// <summary>Heading and body — the only split a sparse document is read as having.</summary>
    private const int SPARSE_FONT_TIER_CLUSTER_COUNT = 2;
    /// <summary>Points either side of a tier centroid that still count as that tier.</summary>
    private const float SPARSE_FONT_TIER_TOLERANCE = 0.5f;
    /// <summary>A tier repeated at the top of several pages is peer sections, not a unique
    /// document title, so H1 stays reserved for a title.</summary>
    private const byte SPARSE_REPEATED_TIER_HEADING_LEVEL = 2;
    /// <summary>Pages that must repeat an H2 tier before title inference is suppressed.</summary>
    private const int SPARSE_PEER_HEADING_MIN_PAGES = 2;
    /// <summary>Font tolerance for the repeated-peer-tier test.</summary>
    private const float SPARSE_PEER_HEADING_FONT_TOLERANCE = 0.5f;
    internal const int MAX_BOLD_HEADING_WORD_COUNT = 12;

    /// <summary>Longest a run may be and still be read as a page number (Rust
    /// <c>MAX_PAGE_NUMBER_WORD_COUNT</c>).</summary>
    private const int MAX_PAGE_NUMBER_WORD_COUNT = 10;

    /// <summary>Points either side of a baseline that still count as the same visual line
    /// (Rust <c>INLINE_STYLE_BASELINE_TOLERANCE</c>).</summary>
    private const float INLINE_STYLE_BASELINE_TOLERANCE = 0.5f;

    /// <summary>Forward gap, as a multiple of the font size, that an inline style run may open
    /// before it reads as a separate block (Rust <c>INLINE_STYLE_MAX_FORWARD_GAP_FONT_FACTOR</c>).</summary>
    private const float INLINE_STYLE_MAX_FORWARD_GAP_FONT_FACTOR = 1.0f;

    /// <summary>Overlap the same way: font metrics let adjacent runs overlap slightly
    /// (Rust <c>INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR</c>).</summary>
    private const float INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR = 0.15f;

    /// <summary>Minimum horizontal gap between two same-line segments, as a fraction of the
    /// trailing segment's font size, that reads as a word space rather than a kerning-run split
    /// of one word. Matches pdf_oxide's own span-joining convention; zero and negative gaps stay
    /// joined (Rust <c>SEGMENT_GAP_SPACE_RATIO</c>).</summary>
    private const float SEGMENT_GAP_SPACE_RATIO = 0.15f;

    private const float PARAGRAPH_GAP_HEIGHT_FACTOR = 1.5f;

    /// <summary>
    /// Multiple of the page's own body leading that a baseline-to-baseline advance must reach
    /// to count as a paragraph break.
    /// </summary>
    /// <remarks>
    /// <see cref="PARAGRAPH_GAP_HEIGHT_FACTOR"/> measures the whitespace band between two lines
    /// against the glyph height, which makes it blind to the commonest paragraph separator there
    /// is. With glyph height <c>h</c> and leading <c>L</c>, single spacing leaves a band of
    /// <c>L - h</c> and a blank line leaves <c>2L - h</c>; for the usual <c>L</c> of 1.1–1.3
    /// <c>h</c> that blank line is only 1.2–1.6 <c>h</c>, so a 1.5<c>h</c> band threshold asks
    /// for more vertical space than a blank line actually provides. Comparing the advance to the
    /// leading is scale-free instead: a blank line doubles the advance. The two rules are OR-ed,
    /// so no break the band rule already finds is lost.
    /// </remarks>
    private const float PARAGRAPH_BREAK_LEADING_MULTIPLE = 1.5f;

    /// <summary>
    /// Largest baseline-to-baseline gap, as a multiple of the larger paragraph's dominant font
    /// size, that a continuation merge will cross. A wrapped line sits roughly one line-height
    /// below its predecessor (leading is typically 1.0–1.6× the font size); several times that
    /// means the two paragraphs come from spatially distinct regions of the page.
    /// </summary>
    private const float MAX_CONTINUATION_LINE_GAP_MULTIPLE = 3.0f;

    // hierarchy clustering constants
    private const int KMEANS_MAX_ITERATIONS = 100;
    private const float KMEANS_CONVERGENCE_THRESHOLD = 0.01f;

    private const float REDRAWN_MIN_TOLERANCE_PTS = 1.0f;
    private const int REDRAWN_LOOKBACK = 8;

    // ── Entry point ────────────────────────────────────────────────────────────

    /// <summary>Build a structured InternalDocument from per-page segments.
    /// Returns null when no elements were produced (caller falls back to flat text).</summary>
    /// <param name="ruledTables">Tables the ruling-line tiers already found, keyed by
    /// their page number. A page they cover keeps them instead of re-deriving a grid
    /// from text geometry, matching the tier priority in the flat table list.</param>
    /// <param name="outlineEntries">The document's bookmarks, if any. A heading the
    /// font-size classifier could not see is recovered from the outline item that names it.</param>
    /// <param name="ruledCoverage">
    /// Every page region the ruled tiers read, where that is not the same thing as
    /// <paramref name="ruledTables"/>. A table joined across a page break is one table filed
    /// under the page it starts on, so the pages its rest came from carry no table of their own
    /// — but their words are still a table's, and taking them out of the paragraph stream is
    /// what stops the same rows coming out a second time as prose. Defaults to
    /// <paramref name="ruledTables"/>, which is what it is whenever nothing was joined.
    /// </param>
    public static InternalDocument? Build(
        List<List<SegmentData>> allPageSegments, int kClusters = 4, List<Table>? ruledTables = null,
        List<PdfOutlineEntry>? outlineEntries = null, List<Table>? ruledCoverage = null)
    {
        int pageCount = allPageSegments.Count;

        // Fragments that row-gap region clustering split out of one physical table are joined
        // back together on the way in, as upstream does at the head of
        // `extract_document_structure_from_segments`. The emitted `tables[]` array is built
        // from the unstitched list, so this must stay local to the structure pipeline.
        if (ruledTables is not null)
        {
            try { ruledTables = PdfTableStitch.StitchFragmentedTables(ruledTables, allPageSegments); }
            catch (Exception) when (Cancel.NotCancelling) { }
        }
        ruledCoverage ??= ruledTables;

        // Text-repair evidence is document-scoped and must be gathered before any page's
        // segments are consumed: a witness on one page can be the sole evidence for a repair
        // decision on another.
        var witnesses = PdfTextRepairWitnesses.Collect(allPageSegments);

        var headingMap = BuildHeadingMap(allPageSegments, kClusters);
        float? docBodyFontSize = null;
        foreach (var (fs, lvl) in headingMap) { if (lvl is null) { docBodyFontSize = fs; break; } }

        var pageHeights = new float[pageCount];
        for (int i = 0; i < pageCount; i++)
        {
            float max = 0f;
            foreach (var s in allPageSegments[i]) max = Math.Max(max, s.Y + s.Height);
            pageHeights[i] = Math.Max(max, 792.0f);
        }

        // Table regions are recovered from text geometry (upstream's fallback for what its layout
        // detector misses, and the whole path without one), then their words are taken out of the
        // paragraph stream so a grid does not also come out as prose.
        var tablesByPage = new List<List<Table>>(pageCount);
        // What a page's tables cover, which is only a different list where a table was joined
        // across a page break: the page its rest came from has coverage and no table.
        var coverageByPage = new List<List<Table>>(pageCount);
        for (int i = 0; i < pageCount; i++)
        {
            List<Table> pageTables;
            uint pageNumber = (uint)(i + 1);
            if (ruledCoverage is not null)
            {
                var covered = ruledCoverage.FindAll(t => t.PageNumber == pageNumber);
                if (covered.Count > 0)
                {
                    tablesByPage.Add(ruledTables?.FindAll(t => t.PageNumber == pageNumber) ?? covered);
                    coverageByPage.Add(covered);
                    continue;
                }
            }
            try
            {
                var words = PdfTableReconstruct.SegmentsToWords(allPageSegments[i], pageHeights[i]);
                var hints = PdfLayoutTables.DetectGeometricTableHints(words, pageHeights[i]);
                pageTables = hints.Count == 0
                    ? new List<Table>()
                    : PdfLayoutTables.ExtractTablesFromLayoutHints(
                        words, hints, i, pageHeights[i], 0.5f,
                        allowSingleColumn: false, prevalidatedColumns: true);
            }
            catch (Exception) when (Cancel.NotCancelling) { pageTables = new List<Table>(); }
            tablesByPage.Add(pageTables);
            coverageByPage.Add(pageTables);
        }

        var allPageParagraphs = new List<List<PdfParagraph>>(pageCount);
        for (int i = 0; i < pageCount; i++)
        {
            // Gap detection reads the page as it was drawn, before the table words are
            // taken out: a band that a grid occupies still separates the prose above it from
            // the prose below, and measuring the gaps on the thinned list would merge the two
            // across the hole the table left behind (`compute_paragraph_gap_ys` is called on
            // the unfiltered page segments upstream, `filter_segments_by_table_bboxes` only
            // afterwards).
            var gapYs = ComputeParagraphGapYs(allPageSegments[i]);
            var segs = OrderSegmentsInReadingFrames(
                FilterSegmentsByTableBboxes(allPageSegments[i], coverageByPage[i]));
            var paras = BlocksToParagraphs(segs, headingMap, gapYs);
            // Segment-level repair runs here, before paragraphs are merged, because the
            // continuation and dehyphenation rules read the last and first characters of
            // neighbouring segments — a trailing soft hyphen or control character left in place
            // would be read as ordinary text and change the decision.
            ApplyToAllSegments(paras, text => PdfTextRepair.RepairSegment(text, witnesses));
            DehyphenateParagraphs(paras, witnesses);
            SplitEmbeddedListItems(paras);
            SynchronizeParagraphTextMetadata(paras);
            MergeContinuationParagraphs(paras);
            SynchronizeParagraphTextMetadata(paras);
            DemoteStructureAnnotationHeadings(paras);
            RetainPageFurnitureSafely(paras);
            allPageParagraphs.Add(paras);
        }

        RefineHeadingHierarchy(allPageParagraphs);
        DemoteUnnumberedSubsections(allPageParagraphs);
        DemoteHeadingRuns(allPageParagraphs);
        SplitColonSemicolonRunInLists(allPageParagraphs);

        // strip_repeating_text default true
        MarkCrossPageRepeatingText(allPageParagraphs, pageHeights);
        MarkCrossPageRepeatingShortText(allPageParagraphs);
        MarkArxivNoise(allPageParagraphs);
        if (outlineEntries is { Count: > 0 }) RecoverHeadingsFromOutline(allPageParagraphs, outlineEntries);
        // After heading recovery, so recovered headings are excluded, and immediately before the
        // deletion pass it feeds.
        MarkValidatedPageNumbers(allPageParagraphs, pageHeights);
        foreach (var page in allPageParagraphs) RetainPageFurnitureSafely(page);
        DeduplicateParagraphs(allPageParagraphs);
        CompactFinalHeadingHierarchy(allPageParagraphs);
        PdfBodySizeBoldHeadings.Promote(allPageParagraphs, docBodyFontSize);

        var doc = AssembleInternalDocument(allPageParagraphs, tablesByPage);

        // Stage 5: element-level text normalization.
        foreach (var elem in doc.Elements)
        {
            if (string.IsNullOrEmpty(elem.Text)) continue;
            string t = RepairContextualLigatures(elem.Text);
            t = ExpandLigaturesWithSpaceAbsorption(t);
            t = NormalizeUnicodeText(t);
            elem.Text = t;
        }

        if (doc.Elements.Count == 0) return null;
        return doc;
    }

    /// <summary>Convert a page's assembled visual lines to segments. Line assembly (spacing,
    /// glyph-fragmentation rebuild) is done by PdfPageText.BuildLineSegments so the text
    /// matches the plain path exactly; here we only wrap it as SegmentData.</summary>
    public static List<SegmentData> SegmentsFromLines(List<PdfPageText.LineSeg> lines)
    {
        var segs = new List<SegmentData>();
        foreach (var l in lines)
        {
            if (string.IsNullOrWhiteSpace(l.Text)) continue;
            segs.Add(new SegmentData
            {
                Text = l.Text,
                X = (float)l.X,
                Y = (float)l.Y,
                Width = (float)l.Width,
                Height = (float)l.Height,
                FontSize = (float)l.FontSize,
                IsBold = l.IsBold,
                IsItalic = l.IsItalic,
                IsMonospace = l.IsMonospace,
                BaselineY = (float)l.Y,
            });
        }
        return DedupeRedrawnSegments(segs);
    }

    /// <summary>
    /// Collapse re-drawn runs: identical text at overlapping positions. A PDF fakes bold by
    /// drawing the same run twice at a small offset, so the survivor absorbs its duplicates'
    /// bold and italic signal — the double draw <em>is</em> the boldness cue.
    /// </summary>
    internal static List<SegmentData> DedupeRedrawnSegments(List<SegmentData> segments)
    {
        var kept = new List<SegmentData>(segments.Count);
        foreach (var seg in segments)
        {
            int windowStart = Math.Max(0, kept.Count - REDRAWN_LOOKBACK);
            SegmentData? prev = null;
            for (int i = windowStart; i < kept.Count; i++)
            {
                var p = kept[i];
                float dxTol = Math.Max(Math.Min(p.Width, seg.Width) * 0.5f, REDRAWN_MIN_TOLERANCE_PTS);
                float dyTol = Math.Max(Math.Min(p.Height, seg.Height) * 0.5f, REDRAWN_MIN_TOLERANCE_PTS);
                if (p.HasSameRotation(seg)
                    && p.Text == seg.Text && Math.Abs(p.X - seg.X) <= dxTol && Math.Abs(p.Y - seg.Y) <= dyTol)
                {
                    prev = p; break;
                }
            }
            if (prev != null)
            {
                prev.IsBold |= seg.IsBold;
                prev.IsItalic |= seg.IsItalic;
                if (seg.FontSize > prev.FontSize) prev.FontSize = seg.FontSize;
                continue;
            }
            kept.Add(seg);
        }
        return kept;
    }

    // ── Font clustering (hierarchy/clustering.rs) ────────────────────────────────

    private sealed class FontSizeCluster { public float Centroid; public List<int> MemberTextLens = new(); }

    /// <summary>One block of the font-size clustering input: a segment's size and the byte
    /// length of its text, which is what decides which cluster is the body (Rust
    /// <c>TextBlock</c>, of which clustering only ever reads <c>font_size</c> and
    /// <c>text.len()</c>).</summary>
    private readonly record struct FontBlock(float FontSize, int TextLen);

    private static List<(float centroid, byte? level)> BuildHeadingMap(List<List<SegmentData>> allPageSegments, int kClusters)
    {
        // Each non-empty segment is one block. The text length travels with it because
        // `AssignHeadingLevelsSmart` picks the body cluster by character mass: drop it and every
        // cluster ties at zero, the tie falls to the smallest font, and every larger run in the
        // document is promoted to a heading (Rust `build_heading_map`, `~keep` comment).
        var blockFonts = new List<FontBlock>();
        foreach (var page in allPageSegments)
            foreach (var seg in page)
                if (!string.IsNullOrWhiteSpace(seg.Text)) blockFonts.Add(new FontBlock(seg.FontSize, Utf8Len(seg.Text)));

        if (blockFonts.Count == 0) return new();

        int paragraphCount = blockFonts.Count;

        if (paragraphCount < MIN_BLOCKS_FOR_FONT_HEADING)
        {
            // A document too small for font clustering can still be structured: a handful of
            // pages that each open at the same larger size are peer sections, and that repetition
            // is evidence the block count alone cannot supply.
            var sparse = SparseMultiPageHeadingMap(allPageSegments, blockFonts);
            if (sparse is not null) return sparse;

            // Sparsity gate: too few text blocks to establish a reliable body-font baseline.
            // Return a body-only map and skip both k-means heading promotion and the fallback
            // title promotion, so a lone larger line on a cover, title or one-line document is
            // not over-promoted to a heading — the bold pass will still call it an H2 if it
            // looks like one.
            var bodyOnly = ClusterFontSizes(blockFonts, 1);
            return bodyOnly.Select(c => (c.Centroid, (byte?)null)).ToList();
        }

        int effectiveK = paragraphCount < 20 ? Math.Min(kClusters, Math.Max(2, paragraphCount / 4)) : kClusters;

        var clusters = ClusterFontSizes(blockFonts, effectiveK);
        var map = AssignHeadingLevelsSmart(clusters, MIN_HEADING_FONT_RATIO);

        // Rust has no cluster-level H1 fallback: its equivalent is the paragraph-level rescue in
        // RefineHeadingHierarchy, which is gated on the document having enough blocks to judge.
        // Without the same gate a one-paragraph document promotes its only text to H1, where
        // Rust leaves the cluster path empty and the bold pass calls it H2.
        bool hasAnyHeading = map.Any(m => m.level.HasValue);
        if (!hasAnyHeading && allPageSegments.Count > 0 && blockFonts.Count >= MIN_BLOCKS_FOR_FONT_HEADING)
        {
            float? firstSegFont = null;
            foreach (var s in allPageSegments[0])
                if (!string.IsNullOrWhiteSpace(s.Text)) { firstSegFont = s.FontSize; break; }
            if (firstSegFont.HasValue)
            {
                var sizes = blockFonts.Select(b => b.FontSize).OrderBy(f => f).ToList();
                float median = sizes.Count == 0 ? 0f : sizes[sizes.Count / 2];
                if (median > 0f && firstSegFont.Value >= median * 1.2f)
                {
                    for (int i = 0; i < map.Count; i++)
                        if (Math.Abs(map[i].centroid - firstSegFont.Value) < 0.5f) { map[i] = (map[i].centroid, 1); break; }
                }
            }
        }
        return map;
    }

    /// <summary>
    /// A heading map for documents below <see cref="MIN_BLOCKS_FOR_FONT_HEADING"/> whose pages
    /// each open at the same larger font tier.
    /// </summary>
    /// <remarks>
    /// The block floor exists because a lone large line is more often display prose than a
    /// section heading. Repetition breaks that tie: when two or more pages begin at the same
    /// enlarged size, those lines are peer sections and the size is a heading tier, however few
    /// blocks the document has. They are peers, not a title, so they get level 2 and H1 stays
    /// reserved. Ported from Rust <c>sparse_multi_page_heading_map</c>; the port has no
    /// structure-tree path, so every page is a heuristic page.
    /// </remarks>
    private static List<(float centroid, byte? level)>? SparseMultiPageHeadingMap(
        List<List<SegmentData>> allPageSegments, List<FontBlock> blockFonts)
    {
        if (allPageSegments.Count < SPARSE_REPEATED_TIER_MIN_PAGES) return null;

        var clusters = ClusterFontSizes(blockFonts, SPARSE_FONT_TIER_CLUSTER_COUNT);
        if (clusters.Count != SPARSE_FONT_TIER_CLUSTER_COUNT) return null;

        // Two tiers only: any block that sits between them means the sizes are a spread rather
        // than a heading/body split, and the repetition argument no longer holds.
        bool twoNarrowTiers = blockFonts.All(b =>
            !float.IsNaN(b.FontSize) && !float.IsInfinity(b.FontSize)
            && clusters.Any(c => Math.Abs(b.FontSize - c.Centroid) <= SPARSE_FONT_TIER_TOLERANCE));
        if (!twoNarrowTiers) return null;

        float headingFont = clusters[0].Centroid;
        float bodyFont = clusters[1].Centroid;
        if (headingFont - bodyFont <= SPARSE_FONT_TIER_TOLERANCE) return null;
        if (headingFont < bodyFont * MIN_HEADING_FONT_RATIO || headingFont < bodyFont + MIN_HEADING_FONT_GAP)
            return null;

        int repeatedPages = 0;
        foreach (var page in allPageSegments)
        {
            var first = page.FirstOrDefault(s => !string.IsNullOrWhiteSpace(s.Text));
            if (first is null) continue;
            if (!float.IsNaN(first.FontSize) && !float.IsInfinity(first.FontSize)
                && Math.Abs(first.FontSize - headingFont) <= SPARSE_FONT_TIER_TOLERANCE)
                repeatedPages++;
        }
        if (repeatedPages < SPARSE_REPEATED_TIER_MIN_PAGES) return null;

        return clusters
            .Select(c => (c.Centroid,
                Math.Abs(c.Centroid - headingFont) <= SPARSE_FONT_TIER_TOLERANCE
                    ? (byte?)SPARSE_REPEATED_TIER_HEADING_LEVEL
                    : null))
            .ToList();
    }

    private static List<FontSizeCluster> ClusterFontSizes(List<FontBlock> blockFonts, int k)
    {
        if (blockFonts.Count == 0) return new();
        if (k == 0) return new();
        int actualK = Math.Min(k, blockFonts.Count);

        var fontSizes = blockFonts.Select(b => b.FontSize).Where(f => !float.IsNaN(f) && !float.IsInfinity(f)).ToList();
        if (fontSizes.Count == 0)
        {
            // Every block's font size was NaN or infinite (a PDF can produce this via a degenerate
            // text or font matrix), so there is no usable font-size signal at all — the same case
            // as no blocks at all, rather than letting the seeding below index an empty list.
            return new();
        }
        fontSizes.Sort((a, b) => b.CompareTo(a)); // descending
        // dedup within 0.05
        var deduped = new List<float>();
        foreach (var f in fontSizes)
            if (deduped.Count == 0 || Math.Abs(deduped[^1] - f) >= 0.05f) deduped.Add(f);
        fontSizes = deduped;

        var centroids = new List<float>();
        if (fontSizes.Count >= actualK)
        {
            int step = fontSizes.Count / actualK;
            for (int i = 0; i < actualK; i++)
            {
                int idx = i * step;
                centroids.Add(fontSizes[Math.Min(idx, fontSizes.Count - 1)]);
            }
        }
        else
        {
            centroids.AddRange(fontSizes);
            float minFont = fontSizes[^1];
            float maxFont = fontSizes[0];
            float range = maxFont - minFont;
            while (centroids.Count < actualK)
            {
                float t = (float)centroids.Count / (actualK - 1);
                centroids.Add(maxFont - t * range);
            }
            centroids.Sort((a, b) => b.CompareTo(a));
        }

        var allFonts = blockFonts.Select(b => b.FontSize).ToList();
        var prevAssign = new int[allFonts.Count];
        bool firstIter = true;

        for (int iter = 0; iter < KMEANS_MAX_ITERATIONS; iter++)
        {
            var sizeClusters = new List<float>[centroids.Count];
            for (int i = 0; i < centroids.Count; i++) sizeClusters[i] = new List<float>();
            var assign = new int[allFonts.Count];
            for (int fi = 0; fi < allFonts.Count; fi++)
            {
                float size = allFonts[fi];
                float minDist = float.PositiveInfinity; int best = 0;
                for (int i = 0; i < centroids.Count; i++)
                {
                    float d = Math.Abs(size - centroids[i]);
                    if (d < minDist) { minDist = d; best = i; }
                }
                sizeClusters[best].Add(size);
                assign[fi] = best;
            }

            int changed;
            if (firstIter) { firstIter = false; changed = 1; }
            else
            {
                changed = 0;
                for (int i = 0; i < assign.Length; i++) if (assign[i] != prevAssign[i]) changed++;
            }
            prevAssign = assign;
            if (changed == 0) break;

            var newCentroids = new List<float>(actualK);
            for (int i = 0; i < sizeClusters.Length; i++)
            {
                if (sizeClusters[i].Count > 0) newCentroids.Add(sizeClusters[i].Sum() / sizeClusters[i].Count);
                else newCentroids.Add(centroids[i]);
            }
            bool converged = true;
            for (int i = 0; i < centroids.Count; i++)
                if (Math.Abs(centroids[i] - newCentroids[i]) >= KMEANS_CONVERGENCE_THRESHOLD) { converged = false; break; }
            centroids = newCentroids;
            if (converged) break;
        }

        // Final assignment: each block joins its nearest centroid and brings its text length,
        // which is the mass `AssignHeadingLevelsSmart` weighs to find the body cluster.
        var memberLens = new List<int>[centroids.Count];
        for (int i = 0; i < centroids.Count; i++) memberLens[i] = new List<int>();
        foreach (var b in blockFonts)
        {
            float minDist = float.PositiveInfinity; int best = 0;
            for (int i = 0; i < centroids.Count; i++)
            {
                float d = Math.Abs(b.FontSize - centroids[i]);
                if (d < minDist) { minDist = d; best = i; }
            }
            memberLens[best].Add(b.TextLen);
        }

        var result = new List<FontSizeCluster>();
        for (int i = 0; i < actualK; i++)
            if (memberLens[i].Count > 0)
                result.Add(new FontSizeCluster { Centroid = centroids[i], MemberTextLens = memberLens[i] });

        result.Sort((a, b) => b.Centroid.CompareTo(a.Centroid));
        return result;
    }

    /// <remarks>
    /// The threshold is purely a ratio against the body centroid. It used to be the smaller of
    /// that ratio and an absolute 1.5pt gap, which let a candidate through on raw size alone —
    /// unsafe once the same code consumes OCR-derived sizes, and upstream measured no change on
    /// native documents when the absolute term was dropped (228 fixtures, 7963 headings,
    /// byte-identical).
    /// </remarks>
    private static List<(float centroid, byte? level)> AssignHeadingLevelsSmart(List<FontSizeCluster> clusters, float minRatio)
    {
        if (clusters.Count == 0) return new();
        if (clusters.Count == 1) return new() { (clusters[0].Centroid, (byte?)null) };

        // Body is the cluster carrying the most characters, not the smallest font: a document
        // whose captions outnumber its prose still has the prose as body. Rust's `max_by_key`
        // keeps the LAST of equal maxima, so the `>=` here is deliberate.
        int bodyIdx = 0; long bestSum = -1;
        for (int i = 0; i < clusters.Count; i++)
        {
            long sum = clusters[i].MemberTextLens.Sum(x => (long)x);
            if (sum >= bestSum) { bestSum = sum; bodyIdx = i; } // >= → keeps last on ties
        }

        float bodyCentroid = clusters[bodyIdx].Centroid;
        float threshold = bodyCentroid * minRatio;

        var candidates = new List<(int idx, float centroid)>();
        for (int i = 0; i < clusters.Count; i++)
            if (i != bodyIdx && clusters[i].Centroid >= threshold) candidates.Add((i, clusters[i].Centroid));
        candidates.Sort((a, b) => b.centroid.CompareTo(a.centroid));

        const int maxHeadings = 6;
        var result = new List<(float, byte?)>(clusters.Count);
        for (int i = 0; i < clusters.Count; i++)
        {
            if (i == bodyIdx) { result.Add((clusters[i].Centroid, null)); continue; }
            int pos = candidates.FindIndex(c => c.idx == i);
            if (pos >= 0 && pos < maxHeadings) result.Add((clusters[i].Centroid, (byte)(pos + 1)));
            else result.Add((clusters[i].Centroid, null));
        }
        return result;
    }

    // ── Gap detection + paragraph grouping (pipeline.rs) ─────────────────────────

    /// <summary>The axis gap detection stacks lines along: page Y for an upright segment, the
    /// segment's own upright baseline for a rotated one.</summary>
    private static float ParagraphGapAxis(SegmentData segment) =>
        segment.IsUnrotated ? segment.Y : segment.UprightBaseline();

    /// <summary>
    /// Paragraph-gap Y positions for a page. A page carrying rotated runs is split into
    /// maximal same-rotation groups first and each measured in its own frame: a gap between an
    /// upright line and a sideways one is not a paragraph break, it is a change of frame, and
    /// mixing the two axes into one sorted stack invents breaks that are not there (Rust
    /// <c>compute_paragraph_gap_ys</c>).
    /// </summary>
    private static List<float> ComputeParagraphGapYs(List<SegmentData> segments)
    {
        if (segments.Count < 2) return new();
        bool allUpright = true;
        foreach (var s in segments) { if (!s.IsUnrotated) { allUpright = false; break; } }
        if (allUpright) return ComputeParagraphGapYsInSharedFrame(segments, 0, segments.Count);

        var gaps = new List<float>();
        int groupStart = 0;
        for (int index = 1; index <= segments.Count; index++)
        {
            bool endsGroup = index == segments.Count || !segments[index - 1].HasSameRotation(segments[index]);
            if (!endsGroup) continue;
            gaps.AddRange(ComputeParagraphGapYsInSharedFrame(segments, groupStart, index));
            groupStart = index;
        }
        return gaps;
    }

    private static List<float> ComputeParagraphGapYsInSharedFrame(List<SegmentData> segments, int from, int to)
    {
        if (to - from < 2) return new();
        var order = Enumerable.Range(from, to - from).ToList();
        order.Sort((a, b) => ParagraphGapAxis(segments[b]).CompareTo(ParagraphGapAxis(segments[a])));

        var lines = new List<(float top, float bottom, float height, bool mono, float anchor)>();
        foreach (var i in order)
        {
            var seg = segments[i];
            var (segBottom, segTop) = seg.IsUnrotated ? (seg.Y, seg.Y + seg.Height) : seg.UprightCrossExtent();
            float baseline = ParagraphGapAxis(seg);
            float tol = Math.Max(seg.Height * 0.5f, 1.0f);
            if (lines.Count > 0)
            {
                var last = lines[^1];
                if (Math.Abs(baseline - last.anchor) <= tol)
                {
                    lines[^1] = (Math.Max(last.top, segTop), Math.Min(last.bottom, segBottom),
                        Math.Max(last.height, seg.Height), last.mono && seg.IsMonospace, last.anchor);
                    continue;
                }
            }
            lines.Add((segTop, segBottom, seg.Height, seg.IsMonospace, baseline));
        }
        if (lines.Count < 2) return new();

        var heights = lines.Select(l => l.height).OrderBy(h => h).ToList();
        float medianHeight = heights[heights.Count / 2];
        float gapThreshold = medianHeight * PARAGRAPH_GAP_HEIGHT_FACTOR;
        float advanceThreshold = BodyLeading(lines, medianHeight) * PARAGRAPH_BREAK_LEADING_MULTIPLE;

        var gapYs = new List<float>();
        for (int i = 0; i + 1 < lines.Count; i++)
        {
            float gap = lines[i].bottom - lines[i + 1].top;
            float advance = lines[i].anchor - lines[i + 1].anchor;
            if ((gap > gapThreshold || advance > advanceThreshold) && !(lines[i].mono && lines[i + 1].mono))
                gapYs.Add((lines[i].bottom + lines[i + 1].top) / 2.0f);
        }
        return gapYs;
    }

    /// <summary>
    /// The page's body leading, read off its own line pitch: the tightest baseline-to-baseline
    /// advance between consecutive lines, floored at the median line height.
    /// </summary>
    /// <remarks>
    /// The tightest advance is used rather than the median or the mode because a short
    /// block-structured page — a memo of five one-line blocks — has more break-sized advances
    /// than body-sized ones, so any central statistic reports the break spacing as normal and no
    /// break is ever found. The floor is what makes the minimum safe: an advance below one line
    /// height is not a wrapped line at all (stacked accents, a subscript resolved onto its own
    /// band) and must not shrink the estimate enough to split every ordinary line on the page.
    /// </remarks>
    private static float BodyLeading(
        List<(float top, float bottom, float height, bool mono, float anchor)> lines, float medianHeight)
    {
        float tightest = float.PositiveInfinity;
        for (int i = 0; i + 1 < lines.Count; i++)
        {
            float advance = lines[i].anchor - lines[i + 1].anchor;
            if (float.IsFinite(advance) && advance > 0f && advance < tightest) tightest = advance;
        }
        return float.IsFinite(tightest) ? Math.Max(tightest, medianHeight) : medianHeight;
    }

    /// <summary>
    /// Repair the reading order inside each maximal rotated run without touching the upright
    /// stream order or moving anything across a frame boundary.
    /// </summary>
    /// <remarks>
    /// A sideways table or axis label arrives in the order its glyphs were painted, which the
    /// page-axis sort upstream of here cannot fix: its rows run along page X, so a row-band sort
    /// on page Y scatters them. Each rotated run is instead re-sorted in its own upright frame —
    /// visual lines by descending upright baseline, then along each line's own advance axis —
    /// while upright runs are handed back untouched, byte for byte (Rust
    /// <c>order_segments_in_reading_frames</c>).
    /// </remarks>
    private static List<SegmentData> OrderSegmentsInReadingFrames(List<SegmentData> segments)
    {
        bool anyRotated = false;
        foreach (var s in segments) { if (!s.IsUnrotated) { anyRotated = true; break; } }
        if (!anyRotated) return segments;

        var groups = new List<List<SegmentData>>();
        foreach (var segment in segments)
        {
            if (groups.Count > 0 && groups[^1][0].HasSameRotation(segment)) groups[^1].Add(segment);
            else groups.Add(new List<SegmentData> { segment });
        }

        var ordered = new List<SegmentData>(segments.Count);
        foreach (var group in groups)
        {
            if (group[0].IsUnrotated) ordered.AddRange(group);
            else ordered.AddRange(OrderRotatedSegmentGroup(group));
        }
        return ordered;
    }

    private static List<SegmentData> OrderRotatedSegmentGroup(List<SegmentData> segments)
    {
        var work = new List<SegmentData>(segments);
        work.Sort((first, second) =>
        {
            int byBaseline = second.UprightBaseline().CompareTo(first.UprightBaseline());
            if (byBaseline != 0) return byBaseline;
            return first.UprightAdvanceExtent().Start.CompareTo(second.UprightAdvanceExtent().Start);
        });

        var visualLines = new List<List<SegmentData>>();
        foreach (var segment in work)
        {
            bool belongsToLastLine = false;
            if (visualLines.Count > 0)
            {
                var anchor = visualLines[^1][0];
                float tolerance =
                    Math.Max(Math.Max(anchor.Height, segment.Height), anchor.FontSize * 0.5f) * 0.5f;
                belongsToLastLine =
                    Math.Abs(anchor.UprightBaseline() - segment.UprightBaseline()) <= tolerance;
            }
            if (belongsToLastLine) visualLines[^1].Add(segment);
            else visualLines.Add(new List<SegmentData> { segment });
        }

        var result = new List<SegmentData>(work.Count);
        foreach (var line in visualLines)
        {
            line.Sort((first, second) =>
                first.UprightAdvanceExtent().Start.CompareTo(second.UprightAdvanceExtent().Start));
            result.AddRange(line);
        }
        return result;
    }

    private static List<PdfParagraph> BlocksToParagraphs(List<SegmentData> lines, List<(float centroid, byte? level)> headingMap, List<float> gapYs)
    {
        if (lines.Count == 0) return new();
        float avgGap = PrecomputeAvgGap(headingMap);

        var paragraphs = new List<PdfParagraph>();
        var current = new List<SegmentData>();
        bool currentIsSingleVisualLine = true;

        for (int lineIdx = 0; lineIdx < lines.Count; lineIdx++)
        {
            var line = lines[lineIdx];
            bool shouldBreak;
            if (current.Count == 0) shouldBreak = false;
            else
            {
                var prev = current[^1];
                bool fontChange = Math.Abs(line.FontSize - prev.FontSize) > 1.5f;
                // A change of weight only ends the paragraph when it is not an inline run
                // continuing the same visual line — a bold lead-in and the prose after it are one
                // paragraph with two styled runs, not two paragraphs.
                bool boldChange = line.IsBold != prev.IsBold
                    && !IsInlineStyleTransition(currentIsSingleVisualLine, prev, line);
                // A run drawn in a different frame always begins a new element: it is a
                // sideways caption or axis label, never a wrapped continuation of the prose
                // above it, and its baseline is not even on the same axis.
                bool rotationChange = !line.HasSameRotation(prev);
                bool startsNewLine = rotationChange
                    || Math.Abs(line.UprightBaseline() - prev.UprightBaseline()) > INLINE_STYLE_BASELINE_TOLERANCE;
                bool hasSameLineFollower = lineIdx + 1 < lines.Count
                    && lines[lineIdx + 1].HasSameRotation(line)
                    && Math.Abs(lines[lineIdx + 1].UprightBaseline() - line.UprightBaseline())
                        <= INLINE_STYLE_BASELINE_TOLERANCE;
                bool isList = startsNewLine
                    && (LooksLikeListItem(line.Text) || (hasSameLineFollower && IsBareListMarker(line.Text)));
                // A numbered section heading always begins a new element. Without this term a run
                // of same-size, same-weight, evenly-spaced headings gives the grouper no boundary
                // at all — `LooksLikeListItem` deliberately declines numbered section headings —
                // and the whole run collapses into one paragraph. The tighter
                // `IsNumberedSectionHeading` is used so prose opening on a bare year does not
                // break its own paragraph.
                bool startsSection = startsNewLine && IsNumberedSectionHeading(line.Text);
                // A numbered section heading also always ENDS the element it opens: nothing else
                // distinguishes a heading from the body text that follows it when both share font
                // size, weight, role and line spacing. `startsSection` above only fires while
                // classifying the heading's own line and cannot see forward to close it; this
                // looks backward at `prev` instead. Restricting to a single accumulated line
                // scopes the break to the line directly after the heading, so a paragraph already
                // several lines long is untouched, and `HeadingWrapsOnto` exempts a heading that
                // is itself still wrapping onto its next physical line.
                bool followsSection = startsNewLine
                    && current.Count == 1
                    && IsNumberedSectionHeading(prev.Text)
                    && !HeadingWrapsOnto(prev, line);
                bool crossedGap = false;
                foreach (var gapY in gapYs)
                {
                    float previousBaseline = prev.UprightBaseline(), currentBaseline = line.UprightBaseline();
                    float upper, lower;
                    if (previousBaseline > currentBaseline) { upper = previousBaseline; lower = currentBaseline; }
                    else { upper = currentBaseline; lower = previousBaseline; }
                    if (gapY < upper && gapY > lower) { crossedGap = true; break; }
                }
                shouldBreak = rotationChange || fontChange || boldChange || isList
                    || startsSection || followsSection || crossedGap;
            }

            if (shouldBreak && current.Count > 0)
            {
                var para = FinalizeParagraph(current, headingMap, avgGap);
                if (para != null) paragraphs.Add(para);
                current = new List<SegmentData>();
                currentIsSingleVisualLine = true;
            }
            if (current.Count > 0)
                currentIsSingleVisualLine &= line.HasSameRotation(current[0])
                    && Math.Abs(line.UprightBaseline() - current[0].UprightBaseline())
                        <= INLINE_STYLE_BASELINE_TOLERANCE;
            current.Add(line);
        }
        if (current.Count > 0)
        {
            var para = FinalizeParagraph(current, headingMap, avgGap);
            if (para != null) paragraphs.Add(para);
        }
        return paragraphs;
    }

    /// <summary>
    /// Whether a change of weight or slant is an inline run continuing the same visual line
    /// rather than a structural boundary (Rust <c>is_inline_style_transition</c>).
    /// </summary>
    /// <remarks>
    /// PDF glyph runs can overlap slightly through font metrics, so a small negative advance gap
    /// still reads as adjacency. A larger overlap, a run that starts to the left of the one
    /// before it, or a wide forward gap all remain boundaries.
    /// </remarks>
    private static bool IsInlineStyleTransition(bool currentIsSingleVisualLine, SegmentData previous, SegmentData next)
    {
        if (!currentIsSingleVisualLine || previous.IsMonospace || next.IsMonospace) return false;
        if (!previous.HasSameRotation(next)) return false;
        if (!float.IsFinite(previous.FontSize) || !float.IsFinite(next.FontSize)
            || previous.FontSize <= 0f || next.FontSize <= 0f
            || !float.IsFinite(previous.UprightBaseline()) || !float.IsFinite(next.UprightBaseline())
            || !float.IsFinite(previous.X) || !float.IsFinite(next.X)
            || !float.IsFinite(previous.Width) || !float.IsFinite(next.Width)
            || previous.Width < 0f || next.Width < 0f)
            return false;
        if (Math.Abs(next.UprightBaseline() - previous.UprightBaseline()) > INLINE_STYLE_BASELINE_TOLERANCE)
            return false;

        float fontSize = Math.Max(previous.FontSize, next.FontSize);
        var (previousStart, previousEnd) = previous.UprightAdvanceExtent();
        float nextStart = next.UprightAdvanceExtent().Start;
        float advanceGap = nextStart - previousEnd;
        return nextStart >= previousStart
            && advanceGap >= -(fontSize * INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR)
            && advanceGap <= fontSize * INLINE_STYLE_MAX_FORWARD_GAP_FONT_FACTOR;
    }

    private static float PrecomputeAvgGap(List<(float centroid, byte? level)> headingMap)
    {
        if (headingMap.Count <= 1) return float.PositiveInfinity;
        var cs = headingMap.Select(h => h.centroid).OrderBy(c => c).ToList();
        var gaps = new List<float>();
        for (int i = 0; i + 1 < cs.Count; i++) gaps.Add(Math.Abs(cs[i + 1] - cs[i]));
        if (gaps.Count == 0) return float.PositiveInfinity;
        return gaps.Sum() / gaps.Count;
    }

    /// <summary>Centroid of the body cluster — the one the heading map left unlevelled.</summary>
    private static float BodyFontSize(List<(float centroid, byte? level)> headingMap)
    {
        foreach (var (c, l) in headingMap) if (l is null) return c;
        return 0f;
    }

    private static byte? FindHeadingLevel(float fontSize, List<(float centroid, byte? level)> headingMap, float avgGap)
    {
        if (headingMap.Count == 0) return null;
        if (headingMap.Count == 1) return headingMap[0].level;
        float bestDist = float.PositiveInfinity; byte? bestLevel = null;
        foreach (var (centroid, level) in headingMap)
        {
            float d = Math.Abs(fontSize - centroid);
            if (d < bestDist) { bestDist = d; bestLevel = level; }
        }
        if (bestDist > MAX_HEADING_DISTANCE_MULTIPLIER * avgGap) return null;
        return bestLevel;
    }

    private static List<PdfLine> ReconstructPdfLines(List<SegmentData> segments)
    {
        var lines = new List<PdfLine>();
        if (segments.Count == 0) return lines;
        float currentBaseline = segments[0].UprightBaseline();
        float currentRotation = segments[0].RotationDegrees;
        var cur = new List<SegmentData>();

        void Flush()
        {
            if (cur.Count == 0) return;
            float dom = 0f;
            foreach (var s in cur)
            {
                float b = s.FontSize;
                if (dom > 0f && b > dom / 2f && b < dom * 2f) dom = (dom + b) / 2f;
                else dom = Math.Max(dom, b);
            }
            bool isBold = cur.Count(s => s.IsBold) > cur.Count / 2;
            bool isMono = cur.All(s => s.IsMonospace);
            lines.Add(new PdfLine { Segments = cur.Select(s => s.Clone()).ToList(), BaselineY = currentBaseline, DominantFontSize = dom, IsBold = isBold, IsMonospace = isMono });
        }

        foreach (var seg in segments)
        {
            bool sameRotation = MathF.Abs(seg.RotationDegrees - currentRotation) <= 1.1920929e-7f;
            float segmentBaseline = seg.UprightBaseline();
            if (!sameRotation || Math.Abs(segmentBaseline - currentBaseline) > 0.5f)
            {
                Flush();
                currentBaseline = segmentBaseline;
                currentRotation = seg.RotationDegrees;
                cur = new List<SegmentData>();
            }
            cur.Add(seg.Clone());
        }
        Flush();
        return lines;
    }

    /// <summary>
    /// The flat text a paragraph's segments read as, used for classification. An all-upright
    /// paragraph is simply its segments newline-joined; once a frame change is in play the
    /// separator has to be decided per pair, because "the next segment is lower down the page"
    /// stops meaning "the next line" (Rust <c>paragraph_text</c>).
    /// </summary>
    private static string ParagraphText(List<SegmentData> lines)
    {
        bool allUpright = true;
        foreach (var s in lines) { if (!s.IsUnrotated) { allUpright = false; break; } }
        if (allUpright) return string.Join("\n", lines.Select(l => l.Text));

        var text = new StringBuilder();
        SegmentData? previous = null;
        foreach (var segment in lines)
        {
            if (previous is not null)
            {
                if (!previous.HasSameRotation(segment)) text.Append("\n\n");
                else
                {
                    float effHeight = Math.Max(Math.Max(previous.Height, segment.Height), segment.FontSize * 0.5f);
                    bool sameLine =
                        Math.Abs(previous.UprightBaseline() - segment.UprightBaseline()) < effHeight * 0.5f;
                    if (sameLine)
                    {
                        string previousWord = LastWhitespaceSeparatedWord(previous.Text);
                        string nextWord = FirstWhitespaceSeparatedWord(segment.Text);
                        if (!(text.Length > 0 && char.IsWhiteSpace(text[^1]))
                            && !(segment.Text.Length > 0 && char.IsWhiteSpace(segment.Text[0]))
                            && SegmentsNeedSpace(previous, previousWord, segment, nextWord))
                            text.Append(' ');
                    }
                    else text.Append('\n');
                }
            }
            text.Append(segment.Text);
            previous = segment;
        }
        return text.ToString();
    }

    private static string LastWhitespaceSeparatedWord(string text)
    {
        var parts = text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
        return parts.Length > 0 ? parts[^1] : "";
    }

    private static string FirstWhitespaceSeparatedWord(string text)
    {
        var parts = text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
        return parts.Length > 0 ? parts[0] : "";
    }

    private static PdfParagraph? FinalizeParagraph(List<SegmentData> lines, List<(float centroid, byte? level)> headingMap, float avgGap)
    {
        if (lines.Count == 0) return null;
        string text = ParagraphText(lines);
        string trimmed = text.Trim();
        if (trimmed.Length == 0) return null;

        var first = lines[0];
        int wordCount = WordCount(trimmed);
        bool isBold = lines.Count(l => l.IsBold) > lines.Count / 2;
        var reconstructed = ReconstructPdfLines(lines);
        float bodyFont = BodyFontSize(headingMap);

        // A bare marker on its own line followed, at the same baseline, by the item's body is
        // one list item split across two segments — the marker alone never reads as a list.
        bool startsWithSplitListMarker = lines.Count > 1
            && IsBareListMarker(first.Text)
            && lines[1].HasSameRotation(first)
            && Math.Abs(lines[1].UprightBaseline() - first.UprightBaseline()) <= INLINE_STYLE_BASELINE_TOLERANCE
            && lines[1].Text.Trim().Length != 0;
        bool isListCandidate = LooksLikeListItem(trimmed) || startsWithSplitListMarker;

        // Shape-only page-number test. It is used here purely to *suppress* heading
        // promotion, which is non-destructive.
        bool pageNumberLike = wordCount <= MAX_PAGE_NUMBER_WORD_COUNT
            && PdfPageNumber.ClassifyPageNumberText(trimmed) is not null;

        // Pass 1: font-size heading.
        byte? headingLevel = FindHeadingLevel(first.FontSize, headingMap, avgGap);
        if (headingLevel.HasValue
            && (wordCount > MAX_HEADING_WORD_COUNT || IsSeparatorText(trimmed) || pageNumberLike))
            headingLevel = null;

        // A bold, short, single-line paragraph is only a heading candidate when its font is
        // also meaningfully larger than the document's body font — the same ratio/gap the
        // font-size clustering path already requires. Without this check any bold one-word
        // line, including body-sized emphasis, gets promoted regardless of scale.
        bool clearsBoldFontGate = bodyFont > 0f
            && first.FontSize >= bodyFont * MIN_HEADING_FONT_RATIO
            && first.FontSize >= bodyFont + MIN_HEADING_FONT_GAP;

        // Pass 2: a bold, oversized, single-line run of a few words. Always H2 — a bold run is
        // evidence of a section head, never of a document title.
        if (headingLevel is null
            && isBold
            && clearsBoldFontGate
            && wordCount >= 1 && wordCount <= 8
            && lines.Count == 1
            && !trimmed.EndsWith('.') && !trimmed.EndsWith(':')
            && !trimmed.EndsWith(',') && !trimmed.EndsWith(';')
            && !trimmed.Contains('@') && !trimmed.Contains('(') && !trimmed.Contains(',')
            && (char.IsUpper(trimmed[0]) || char.IsAsciiDigit(trimmed[0]))
            && !IsSeparatorText(trimmed)
            && !LooksLikeFigureLabel(trimmed))
            headingLevel = 2;

        // Pass 3: an oversized run that also reads as a section marker — numbered, all-caps,
        // or one of the structural words a document divides itself with.
        if (headingLevel is null)
        {
            float minHeadingThreshold = bodyFont * MIN_HEADING_FONT_RATIO;
            if (bodyFont > 0f
                && first.FontSize >= minHeadingThreshold
                && first.FontSize > bodyFont + 0.5f
                && wordCount <= MAX_BOLD_HEADING_WORD_COUNT
                && lines.Count <= 2
                && !trimmed.EndsWith(':')
                && !trimmed.Contains('@')
                && (IsSectionPattern(trimmed) || IsStructuralHeadingWord(trimmed))
                && !IsSeparatorText(trimmed)
                && !LooksLikeFigureLabel(trimmed)
                && !isListCandidate
                && !pageNumberLike)
                headingLevel = 2;
        }

        bool isListItem = headingLevel is null && isListCandidate;
        bool isCodeBlock = headingLevel is null && !isListItem && lines.All(l => l.IsMonospace) && lines.Count >= 2;
        // Page-number furniture is decided document-wide by MarkValidatedPageNumbers, which
        // needs every page in hand: shape alone matches table cells, list markers and footnote
        // references, and furniture under 80 alphanumeric characters is physically deleted.

        int finalWc = ComputeWordCount(trimmed, reconstructed);
        float l0 = lines.Min(l => l.X);
        float b0 = lines.Min(l => l.BaselineY);
        float r0 = lines.Max(l => l.X + l.Width);
        float t0 = lines.Max(l => l.BaselineY + l.Height);

        return new PdfParagraph
        {
            Text = trimmed,
            Lines = reconstructed,
            DominantFontSize = first.FontSize,
            HeadingLevel = headingLevel,
            IsBold = isBold,
            IsListItem = isListItem,
            IsCodeBlock = isCodeBlock,
            IsFormula = false,
            IsPageFurniture = false,
            BlockBbox = (l0, b0, r0, t0),
            WordCount = finalWc,
        };
    }

    private static int WordCount(string s) => s.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).Length;

    private static int ComputeWordCount(string text, List<PdfLine> lines)
    {
        if (text.Length > 0) return WordCount(text);
        int sum = 0;
        foreach (var l in lines) foreach (var s in l.Segments) sum += WordCount(s.Text);
        return sum;
    }

    // ── merge_continuation_paragraphs (paragraphs.rs) ────────────────────────────

    /// <summary>Apply a text repair to every segment of every paragraph, in place.</summary>
    private static void ApplyToAllSegments(List<PdfParagraph> paragraphs, Func<string, string> repair)
    {
        foreach (var para in paragraphs)
        {
            bool changed = false;
            foreach (var line in para.Lines)
                for (int i = 0; i < line.Segments.Count; i++)
                {
                    string repaired = repair(line.Segments[i].Text);
                    if (!ReferenceEquals(repaired, line.Segments[i].Text) && repaired != line.Segments[i].Text)
                    {
                        line.Segments[i].Text = repaired;
                        changed = true;
                    }
                }
            // The cached paragraph text was assembled from the segments; once they move it is
            // stale, and every reader falls back to rejoining the segments.
            if (changed) para.Text = "";
        }
    }

    /// <summary>
    /// Drop the cached paragraph text and refresh the metadata derived from it.
    /// </summary>
    /// <remarks>
    /// Assembly derives both the emitted text and the inline annotation byte ranges from the
    /// segments whenever <c>Text</c> is empty, so an empty cache is what keeps repaired segment
    /// text from diverging from the stale pre-repair string — and it is also what gives a
    /// paragraph its per-run bold and italic spans instead of one annotation over the whole
    /// block. Ports Rust <c>synchronize_paragraph_text_metadata</c>, which the heuristic path
    /// runs after every segment mutation, leaving <c>text</c> empty for the whole pipeline.
    /// </remarks>
    private static void SynchronizeParagraphTextMetadata(List<PdfParagraph> paragraphs)
    {
        foreach (var para in paragraphs)
        {
            para.Text = "";
            para.WordCount = ComputeWordCount("", para.Lines);
        }
    }

    /// <summary>Largest fraction of a paragraph's own right edge a line may fall short of and
    /// still count as reaching the margin.</summary>
    private const float FULL_LINE_FRACTION = 0.85f;

    /// <summary>
    /// Compounds whose hyphen is lexical and must survive a line break. A trailing ASCII hyphen
    /// is otherwise indistinguishable from a discretionary wrap hyphen; matching whole pairs is
    /// deliberately narrower than any prefix or suffix rule, so <c>soft-</c> + <c>ware</c> is
    /// still rejoined.
    /// </summary>
    private static readonly (string Left, string Right)[] PRESERVED_LEXICAL_COMPOUNDS =
    {
        ("cost", "effective"), ("evidence", "based"), ("high", "level"), ("long", "term"),
        ("low", "level"), ("real", "time"), ("short", "term"), ("state", "of-the-art"),
        ("user", "defined"), ("well", "known"),
    };

    private static bool ShouldPreserveLexicalHyphen(
        string trailingWord, string leadingWord, PdfTextRepairWitnesses witnesses)
    {
        static string Trim(string s)
        {
            int start = 0, end = s.Length;
            while (start < end && !char.IsLetterOrDigit(s[start]) && s[start] != '-') start++;
            while (end > start && !char.IsLetterOrDigit(s[end - 1]) && s[end - 1] != '-') end--;
            return s.Substring(start, end - start);
        }
        string left = Trim(trailingWord), right = Trim(leadingWord);
        foreach (var (expectedLeft, expectedRight) in PRESERVED_LEXICAL_COMPOUNDS)
            if (string.Equals(left, expectedLeft, StringComparison.OrdinalIgnoreCase)
                && string.Equals(right, expectedRight, StringComparison.OrdinalIgnoreCase))
                return true;
        // The static list covers common English compounds; a hyphen the document itself writes
        // mid-run elsewhere is the document's own evidence that this one is authored rather than
        // a line-wrap artefact (xberg-io/xberg#1543).
        return witnesses.Hyphens.Contains((left.ToLowerInvariant(), right.ToLowerInvariant()));
    }

    /// <summary>
    /// Rejoin words a line break split across a discretionary hyphen, at the segment level and
    /// before the paragraphs are merged (Rust <c>dehyphenate_paragraphs</c>).
    /// </summary>
    /// <remarks>
    /// The assembly layer already drops a wrap hyphen when it joins two lines' text, but that
    /// happens too late for anything reading the segments themselves — the continuation and
    /// list tests, and every downstream classifier — and it cannot move the second half of the
    /// word onto the first. Here the halves are actually spliced: the trailing segment absorbs
    /// the leading word and the following segment gives it up.
    /// </remarks>
    private static void DehyphenateParagraphs(List<PdfParagraph> paragraphs, PdfTextRepairWitnesses witnesses)
    {
        foreach (var para in paragraphs)
        {
            if (para.IsCodeBlock || para.Lines.Count < 2) continue;
            DehyphenateParagraphLines(para, witnesses);
        }
    }

    private static void DehyphenateParagraphLines(PdfParagraph para, PdfTextRepairWitnesses witnesses)
    {
        float maxRightEdge = 0f;
        foreach (var line in para.Lines)
            foreach (var s in line.Segments)
                maxRightEdge = Math.Max(maxRightEdge, s.X + s.Width);

        // Without usable geometry every boundary is a candidate, gated on the hyphen alone.
        float threshold = maxRightEdge > 0f ? maxRightEdge * FULL_LINE_FRACTION : float.NegativeInfinity;

        for (int i = 0; i + 1 < para.Lines.Count; i++)
        {
            var trailingSegments = para.Lines[i].Segments;
            var leadingSegments = para.Lines[i + 1].Segments;
            if (trailingSegments.Count == 0 || leadingSegments.Count == 0) continue;

            var trailingSeg = trailingSegments[^1];
            // A paragraph line boundary is not always a visual one: inline style and
            // font-resource splitting introduce them mid-line, so a suspended hyphen
            // ("vracht- en verzendkosten") can end one logical line and begin the next while
            // both runs share a baseline. Joining is licensed only across a real line break
            // (xberg-io/xberg#1561).
            if (!CrossesVisualLineBreak(trailingSeg, leadingSegments[0])) continue;
            if (maxRightEdge > 0f && trailingSeg.X + trailingSeg.Width < threshold) continue;

            string trailingText = trailingSeg.Text;
            if (trailingText.Length == 0 || !trailingText.EndsWith('-')) continue;
            string leadingText = leadingSegments[0].Text;
            if (leadingText.Length == 0) continue;

            string leadingWord = FirstWhitespaceSeparatedWord(leadingText);
            if (leadingWord.Length > 0 && char.IsUpper(leadingWord[0])) continue;

            string trailingWord = LastWhitespaceSeparatedWord(trailingText.TrimEnd('-'));
            if (trailingWord.Length > 0 && IsCjkChar(trailingWord[^1])) continue;

            string preservedHyphen = ShouldPreserveLexicalHyphen(trailingWord, leadingWord, witnesses) ? "-" : "";
            string joinedWord = trailingWord + preservedHyphen + leadingWord;

            // The characters dropped off the trailing segment are counted in UTF-8 bytes, as
            // upstream counts them, so a word carrying multi-byte glyphs eats further back into
            // the segment than its character count alone would.
            int drop = System.Text.Encoding.UTF8.GetByteCount(trailingWord) + 1;
            string withoutWord = drop >= trailingSeg.Text.Length
                ? "" : trailingSeg.Text.Substring(0, trailingSeg.Text.Length - drop);
            trailingSeg.Text = withoutWord + joinedWord;

            var leadSeg = leadingSegments[0];
            string rest = leadSeg.Text;
            if (leadingWord.Length > 0)
                while (rest.StartsWith(leadingWord, StringComparison.Ordinal))
                    rest = rest.Substring(leadingWord.Length);
            leadSeg.Text = rest.TrimStart();
        }
    }

    /// <summary>
    /// Break a body paragraph that swallowed a whole bullet list into the list items it is
    /// made of (Rust <c>split_embedded_list_items</c>).
    /// </summary>
    /// <remarks>
    /// A producer that draws its bullets as ordinary glyphs inside the running text leaves the
    /// line grouper nothing to break on, so a five-item list arrives as one paragraph with five
    /// bullet characters in the middle of it. Two bullets are required before the split fires,
    /// because a single one is far more likely to be a mid-sentence separator than a list.
    /// The pieces are rebuilt from text alone and carry no geometry, which is why the pass runs
    /// after everything that measures the page.
    /// </remarks>
    private static void SplitEmbeddedListItems(List<PdfParagraph> paragraphs)
    {
        var old = new List<PdfParagraph>(paragraphs);
        paragraphs.Clear();
        foreach (var para in old)
        {
            if (para.HeadingLevel is not null || para.IsListItem || para.IsCodeBlock || para.IsFormula)
            {
                paragraphs.Add(para);
                continue;
            }

            var pieces = new List<string>();
            foreach (var line in para.Lines)
                foreach (var seg in line.Segments)
                    pieces.Add(seg.Text);
            string fullText = string.Join(" ", pieces);

            int bulletCount = 0;
            foreach (var c in fullText) if (c is '\u2022' or '\u00B7') bulletCount++;
            if (bulletCount < 2) { paragraphs.Add(para); continue; }

            float fontSize = para.DominantFontSize;
            bool isBold = para.IsBold;

            var parts = fullText.Split('\u2022', '\u00B7');
            // A UTF-8 bullet mis-decoded as Latin-1 leaves its lead byte stranded beside the
            // marker; strip it where it lands rather than letting it open a list item.
            string before = parts[0].Trim().TrimEnd('\u00C2').Trim();
            if (before.Length > 0) paragraphs.Add(TextToParagraph(before, fontSize, isBold, false));
            for (int i = 1; i < parts.Length; i++)
            {
                string itemText = parts[i].Trim().TrimStart('\u00C2').TrimEnd('\u00C2').Trim();
                if (itemText.Length > 0) paragraphs.Add(TextToParagraph(itemText, fontSize, isBold, true));
            }
        }
    }

    /// <summary>A paragraph built from text alone: one zero-geometry segment per word.</summary>
    private static PdfParagraph TextToParagraph(string text, float fontSize, bool isBold, bool isListItem)
    {
        var segments = new List<SegmentData>();
        foreach (var word in text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
            segments.Add(new SegmentData { Text = word, FontSize = fontSize, IsBold = isBold });

        var lines = new List<PdfLine>
        {
            new PdfLine { Segments = segments, BaselineY = 0f, DominantFontSize = fontSize, IsBold = isBold },
        };
        return new PdfParagraph
        {
            Text = "",
            Lines = lines,
            DominantFontSize = fontSize,
            IsBold = isBold,
            IsListItem = isListItem,
            WordCount = ComputeWordCount("", lines),
        };
    }

    /// <summary>Structure-tree heading tags that were really parameter metadata.</summary>
    private static readonly string[] SAL_DIRECTION_PREFIXES =
    {
        "__deref__inout", "__deref__out", "__deref__in", "__inout", "__out", "__in",
    };

    private static readonly string[] SAL_DIRECTION_MODIFIERS =
    {
        "opt", "ecount", "bcount", "full", "part", "z", "nz",
    };

    /// <summary>
    /// Demote a heading whose whole text is a source-annotation direction such as
    /// <c>__in</c> or <c>__out_ecount_opt(n)</c> (Rust
    /// <c>demote_structure_annotation_headings</c>).
    /// </summary>
    /// <remarks>
    /// A tagged API reference sometimes tags these annotations as headings. They are parameter
    /// metadata, not document structure. The test is deliberately narrow — the whole paragraph
    /// must be one annotation — and a code block keeps its classification.
    /// </remarks>
    private static void DemoteStructureAnnotationHeadings(List<PdfParagraph> paragraphs)
    {
        foreach (var para in paragraphs)
        {
            if (para.HeadingLevel is null || para.IsCodeBlock) continue;
            if (IsSalDirectionAnnotation(ParagraphPlainText(para).Trim())) para.HeadingLevel = null;
        }
    }

    private static bool IsSalDirectionAnnotation(string text)
    {
        string basePart;
        int open = text.IndexOf('(');
        if (open >= 0)
        {
            if (!text.EndsWith(')')) return false;
            string arguments = text.Substring(open + 1, text.Length - open - 2);
            if (arguments.Length == 0) return false;
            foreach (var c in arguments)
                if (!char.IsAsciiLetterOrDigit(c) && c != '_' && c != ',') return false;
            basePart = text.Substring(0, open);
        }
        else basePart = text;

        foreach (var prefix in SAL_DIRECTION_PREFIXES)
        {
            if (!basePart.StartsWith(prefix, StringComparison.Ordinal)) continue;
            string suffix = basePart.Substring(prefix.Length);
            if (suffix.Length == 0) return true;
            if (suffix[0] != '_') continue;
            string modifiers = suffix.Substring(1);
            if (modifiers.Length == 0) continue;
            bool allKnown = true;
            foreach (var modifier in modifiers.Split('_'))
                if (Array.IndexOf(SAL_DIRECTION_MODIFIERS, modifier) < 0) { allKnown = false; break; }
            if (allKnown) return true;
        }
        return false;
    }

    private static void MergeContinuationParagraphs(List<PdfParagraph> paragraphs)
    {
        if (paragraphs.Count < 2) return;
        var old = new List<PdfParagraph>(paragraphs);
        paragraphs.Clear();
        var current = old[0];
        for (int idx = 1; idx < old.Count; idx++)
        {
            var next = old[idx];
            bool bothBody = current.HeadingLevel is null && next.HeadingLevel is null
                && !current.IsListItem && !next.IsListItem
                && !current.IsCodeBlock && !next.IsCodeBlock
                && !current.IsFormula && !next.IsFormula;
            bool fontsCompatible = Math.Abs(current.DominantFontSize - next.DominantFontSize) < 2.0f;
            // Never merge across a bold-state boundary. A bold run following non-bold prose (or
            // the reverse) is a formatting break — an emphasized heading, a list item's bold
            // lead-in — not a wrapped continuation, and absorbing it buries the heading as inline
            // bold before anything can classify it.
            bool boldCompatible = current.IsBold == next.IsBold;
            bool continuationSignal = !EndsWithSentenceTerminator(current) || StartsWithLowercaseContinuation(next);
            bool sameRotation = ParagraphsShareRotation(current, next);
            bool verticalGapCompatible = BaselinesWithinContinuationGap(current, next);
            // A numbered section heading starts a new element. It does not end in `.?!:;`, so the
            // continuation signal is satisfied by the *previous* heading alone — an `||` boost,
            // not a requirement — and a run of consecutive subsection headings would be rejoined
            // here even after the line grouper split them.
            bool nextStartsSection = StartsNumberedSection(next);
            // The mirror of `nextStartsSection`: a numbered section heading must never be absorbed
            // AS a continuation either. The line grouper already splits a heading from unrelated
            // text that follows it, but that split is undone here unless this pass independently
            // refuses to re-join it — the two passes see none of each other's decisions. The one
            // exception is a heading still wrapping onto its next physical line, which must stay
            // joined; `HeadingWrapsOnto` is the same right-edge test the grouper uses, applied to
            // the boundary segments so both passes agree on the same wrap.
            bool currentStartsSection = StartsNumberedSection(current);
            var boundaryPrev = current.Lines.LastOrDefault()?.Segments.LastOrDefault();
            var boundaryNext = next.Lines.FirstOrDefault()?.Segments.FirstOrDefault();
            bool boundaryIsHeadingWrap = boundaryPrev is not null && boundaryNext is not null
                && HeadingWrapsOnto(boundaryPrev, boundaryNext);
            if (bothBody && fontsCompatible && boldCompatible && continuationSignal
                && sameRotation && verticalGapCompatible && !nextStartsSection
                && (!currentStartsSection || boundaryIsHeadingWrap))
            {
                current.Text = "";
                current.BlockBbox = UnionBlockBbox(current.BlockBbox, next.BlockBbox);
                current.Lines.AddRange(next.Lines);
            }
            else { paragraphs.Add(current); current = next; }
        }
        paragraphs.Add(current);
    }

    /// <summary>Whether the run ending one paragraph and the run opening the next were drawn
    /// in the same frame. Paragraphs without segments are treated as compatible, since no
    /// frame can be read off them (Rust <c>paragraphs_share_rotation</c>).</summary>
    private static bool ParagraphsShareRotation(PdfParagraph current, PdfParagraph next)
    {
        var currentLast = current.Lines.LastOrDefault()?.Segments.LastOrDefault();
        var nextFirst = next.Lines.FirstOrDefault()?.Segments.FirstOrDefault();
        if (currentLast is null || nextFirst is null) return true;
        return currentLast.HasSameRotation(nextFirst);
    }

    /// <summary>
    /// Whether <paramref name="next"/>'s first baseline sits close enough below
    /// <paramref name="current"/>'s last to be a wrapped continuation rather than a spatially
    /// distant block — a recipient block at the top of an invoice and a legal footer at the
    /// bottom satisfy every textual signal and must still never be joined.
    /// </summary>
    /// <remarks>Paragraphs without per-line geometry are allowed to merge, gated by the other
    /// signals, since no vertical distance can be computed for them.</remarks>
    private static bool BaselinesWithinContinuationGap(PdfParagraph current, PdfParagraph next)
    {
        var currentLast = current.Lines.LastOrDefault();
        var nextFirst = next.Lines.FirstOrDefault();
        if (currentLast is null || nextFirst is null) return true;
        if (currentLast.BaselineY == 0f || nextFirst.BaselineY == 0f) return true;
        float gap = Math.Abs(currentLast.BaselineY - nextFirst.BaselineY);
        float lineHeight = Math.Max(Math.Max(current.DominantFontSize, next.DominantFontSize), 1.0f);
        return gap <= lineHeight * MAX_CONTINUATION_LINE_GAP_MULTIPLE;
    }

    /// <summary>Union of two block boxes, so a merged paragraph's box spans all of its text.</summary>
    private static (float L, float B, float R, float T)? UnionBlockBbox(
        (float L, float B, float R, float T)? current, (float L, float B, float R, float T)? next)
    {
        if (current is { } c && next is { } n)
            return (Math.Min(c.L, n.L), Math.Min(c.B, n.B), Math.Max(c.R, n.R), Math.Max(c.T, n.T));
        return current ?? next;
    }

    /// <summary>
    /// Whether a paragraph's first visual line reads as a numbered section heading
    /// ("1.3 Gasinstallatie", "IV. Results", "1. INTRODUCTION").
    /// </summary>
    /// <remarks>The line's segments are rejoined because word-processor output often splits the
    /// numbering into its own run, leaving the leading segment as a bare "1.3".</remarks>
    /// <summary>
    /// Tolerance, as a multiple of font size, on the right-edge alignment that says a heading is
    /// still wrapping onto its next physical line.
    /// </summary>
    private const float HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR = 2.0f;

    /// <summary>
    /// Whether <paramref name="line"/> reads as the wrapped continuation of a heading on
    /// <paramref name="prev"/>, rather than unrelated content handed off to.
    /// </summary>
    /// <remarks>
    /// A wrapped heading line ends at roughly the same right edge as the line it wrapped from.
    /// Both paragraph passes use this same test, so they agree on the same wrap.
    /// </remarks>
    private static bool HeadingWrapsOnto(SegmentData prev, SegmentData line)
    {
        if (!prev.HasSameRotation(line)) return false;
        if (!float.IsFinite(prev.FontSize) || !float.IsFinite(line.FontSize)) return false;

        float prevEnd = prev.UprightAdvanceExtent().End;
        float lineEnd = line.UprightAdvanceExtent().End;
        if (!float.IsFinite(prevEnd) || !float.IsFinite(lineEnd)) return false;

        float tolerance = HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR
                          * Math.Max(Math.Max(prev.FontSize, line.FontSize), 1.0f);
        return Math.Abs(prevEnd - lineEnd) <= tolerance;
    }

    private static bool StartsNumberedSection(PdfParagraph para)
    {
        var firstLine = para.Lines.FirstOrDefault();
        if (firstLine is null) return false;
        string joined = string.Join(" ", firstLine.Segments
            .Select(s => s.Text.Trim())
            .Where(t => t.Length > 0));
        return IsNumberedSectionHeading(joined);
    }

    private static bool StartsWithLowercaseContinuation(PdfParagraph p)
    {
        var seg = p.Lines.FirstOrDefault()?.Segments.FirstOrDefault();
        string t = seg?.Text.TrimStart() ?? "";
        return t.Length > 0 && char.IsLower(t[0]);
    }

    private static bool EndsWithSentenceTerminator(PdfParagraph p)
    {
        var seg = p.Lines.LastOrDefault()?.Segments.LastOrDefault();
        string t = seg?.Text.TrimEnd() ?? "";
        if (t.Length == 0) return false;
        char c = t[^1];
        return c is '.' or '?' or '!' or ':' or ';' or '。' or '？' or '！';
    }

    private static void RetainPageFurnitureSafely(List<PdfParagraph> paragraphs)
    {
        int total = paragraphs.Count;
        int furniture = paragraphs.Count(p => p.IsPageFurniture);
        if (furniture == 0) return;
        if (furniture >= total)
        {
            foreach (var p in paragraphs) p.IsPageFurniture = false;
            return;
        }

        // A page whose "furniture" carries a third of its characters is not a page with
        // furniture on it — the classifier is wrong about something, and deleting on that
        // reading would take real content with it. Stand down entirely.
        long totalAlphanum = paragraphs.Sum(p => (long)ParagraphAlphanumLen(p));
        if (totalAlphanum > 0)
        {
            long furnitureAlphanum = paragraphs.Where(p => p.IsPageFurniture).Sum(p => (long)ParagraphAlphanumLen(p));
            if (furnitureAlphanum * 100 > totalAlphanum * 30)
            {
                foreach (var p in paragraphs) p.IsPageFurniture = false;
                return;
            }
        }

        // Running heads and page numbers are short. Anything longer than this is prose that
        // happens to sit in a margin, and keeping it costs far less than losing it.
        const int MIN_SUBSTANTIVE_CHARS = 80;
        paragraphs.RemoveAll(p => p.IsPageFurniture && ParagraphAlphanumLen(p) <= MIN_SUBSTANTIVE_CHARS);
    }

    /// <summary>Page width assumed when a document yields no usable paragraph geometry —
    /// US Letter portrait, matching the 792pt height fallback.</summary>
    private const float FALLBACK_PAGE_WIDTH_PTS = 612.0f;

    /// <summary>Page height assumed when no entry exists for a page index.</summary>
    private const float FALLBACK_PAGE_HEIGHT_PTS = 792.0f;

    /// <summary>
    /// Mark confirmed running page numbers as furniture.
    /// </summary>
    /// <remarks>
    /// Three independent signals must agree before anything is marked: the shape must be one
    /// the classifier recognises, the paragraph's vertical centre must fall in a margin band,
    /// and the cross-page sequence must reach the deletion threshold. Where confidence falls
    /// short the text is kept — retaining an occasional page number is far cheaper than silently
    /// dropping a table cell. Ports Rust <c>mark_validated_page_numbers</c>.
    /// </remarks>
    private static void MarkValidatedPageNumbers(List<List<PdfParagraph>> allPages, float[] pageHeights)
    {
        float pageWidth = DocumentContentWidth(allPages);
        var sequence = new PdfPageNumber.PageNumberSequence();
        var observations = new List<(int Page, int Paragraph, float YRatio, float XRatio)>();

        // Pass 1: observe every candidate on every page. No deletion decision is taken here —
        // the sequence is not usable until it has seen all pages.
        for (int pageIndex = 0; pageIndex < allPages.Count; pageIndex++)
        {
            Cancel.ThrowIfRequested();
            float pageHeight = pageIndex < pageHeights.Length ? pageHeights[pageIndex] : FALLBACK_PAGE_HEIGHT_PTS;
            var page = allPages[pageIndex];
            for (int paragraphIndex = 0; paragraphIndex < page.Count; paragraphIndex++)
            {
                var (yRatio, xRatio, candidate) = PageNumberObservation(page[paragraphIndex], pageHeight, pageWidth);
                if (candidate is not { } shape) continue;
                sequence.Observe(pageIndex, PdfPageNumber.Band(yRatio), xRatio, shape);
                observations.Add((pageIndex, paragraphIndex, yRatio, xRatio));
            }
        }

        // Pass 2: confirm. Every page has now been observed.
        foreach (var (pageIndex, paragraphIndex, yRatio, xRatio) in observations)
        {
            var band = PdfPageNumber.Band(yRatio);
            if (band == MarginBand.Body) continue;
            if (sequence.ConfidenceAt(pageIndex, band, xRatio)
                < PdfPageNumber.PageNumberSequence.DeletionThreshold) continue;
            allPages[pageIndex][paragraphIndex].IsPageFurniture = true;
        }
    }

    /// <summary>A page-number observation for one paragraph: its normalized position and the
    /// shape candidate, or nulls when the paragraph is not a candidate or carries no usable
    /// geometry — either way it is never deletable.</summary>
    private static (float YRatio, float XRatio, PageNumberCandidate? Candidate) PageNumberObservation(
        PdfParagraph paragraph, float pageHeight, float pageWidth)
    {
        if (paragraph.HeadingLevel is not null
            || paragraph.IsListItem
            || paragraph.IsCodeBlock
            || paragraph.IsPageFurniture
            || paragraph.WordCount > MAX_PAGE_NUMBER_WORD_COUNT)
            return (0f, 0f, null);
        if (PdfPageNumber.ClassifyPageNumberText(ParagraphTextRaw(paragraph).Trim()) is not { } candidate)
            return (0f, 0f, null);
        var (yRatio, xRatio) = ParagraphPositionRatios(paragraph, pageHeight, pageWidth);
        if (float.IsNaN(yRatio) || float.IsNaN(xRatio)) return (0f, 0f, null);
        return (yRatio, xRatio, candidate);
    }

    /// <summary>Normalized position of a paragraph's centre within its page: y 0.0 at the top and
    /// 1.0 at the bottom, so the vertical axis is inverted from PDF space. NaN when the geometry
    /// is unusable.</summary>
    private static (float YRatio, float XRatio) ParagraphPositionRatios(
        PdfParagraph paragraph, float pageHeight, float pageWidth)
    {
        if (!float.IsFinite(pageHeight) || pageHeight <= 0f || !float.IsFinite(pageWidth) || pageWidth <= 0f)
            return (float.NaN, float.NaN);
        if (FiniteParagraphBbox(paragraph) is not { } bbox) return (float.NaN, float.NaN);
        float centreY = (bbox.B + bbox.T) * 0.5f;
        float centreX = (bbox.L + bbox.R) * 0.5f;
        return (Math.Clamp(1f - centreY / pageHeight, 0f, 1f), Math.Clamp(centreX / pageWidth, 0f, 1f));
    }

    /// <summary>The paragraph's geometry, restricted to fully finite boxes: a paragraph assembled
    /// from degenerate font metrics would otherwise normalize to a meaningless position.</summary>
    private static (float L, float B, float R, float T)? FiniteParagraphBbox(PdfParagraph paragraph)
    {
        if (ParagraphGeometryBbox(paragraph) is not { } bbox) return null;
        return float.IsFinite(bbox.L) && float.IsFinite(bbox.B)
            && float.IsFinite(bbox.R) && float.IsFinite(bbox.T) ? bbox : null;
    }

    private static (float L, float B, float R, float T)? ParagraphGeometryBbox(PdfParagraph paragraph)
    {
        if (paragraph.BlockBbox is { } blockBbox) return blockBbox;
        var segments = paragraph.Lines.SelectMany(line => line.Segments).ToList();
        if (segments.Count == 0) return null;
        var first = segments[0];
        float l = first.X, b = Math.Min(first.Y, first.BaselineY);
        float r = first.X + first.Width, t = Math.Max(first.Y + first.Height, first.BaselineY + first.Height);
        foreach (var segment in segments)
        {
            l = Math.Min(l, segment.X);
            b = Math.Min(b, Math.Min(segment.Y, segment.BaselineY));
            r = Math.Max(r, segment.X + segment.Width);
            t = Math.Max(t, Math.Max(segment.Y + segment.Height, segment.BaselineY + segment.Height));
        }
        return (l, b, r, t);
    }

    /// <summary>Widest right edge across the whole document. A document-wide value rather than a
    /// per-page one, because the sequence compares horizontal positions <em>across</em> pages:
    /// with a per-page normalizer a stable footer slot would read as drifting.</summary>
    private static float DocumentContentWidth(List<List<PdfParagraph>> allPages)
    {
        float widest = 0f;
        foreach (var page in allPages)
            foreach (var paragraph in page)
                if (FiniteParagraphBbox(paragraph) is { } bbox && bbox.R > 0f) widest = Math.Max(widest, bbox.R);
        return widest > 0f ? widest : FALLBACK_PAGE_WIDTH_PTS;
    }

    /// <summary>ASCII letters and digits across a paragraph's segments — the measure of how much
    /// real content a paragraph carries (Rust <c>paragraph_alphanum_len</c>).</summary>
    private static int ParagraphAlphanumLen(PdfParagraph para)
    {
        int count = 0;
        foreach (var line in para.Lines)
            foreach (var seg in line.Segments)
                foreach (char c in seg.Text)
                    if (char.IsAsciiLetterOrDigit(c)) count++;
        return count;
    }

    // ── outline-driven heading recovery (pipeline.rs) ────────────────────────────

    /// <summary>Depth added to an outline item's depth when nothing calibrates the offset.</summary>
    private const int DEFAULT_OUTLINE_HEADING_OFFSET = 2;

    /// <summary>Headings that must already agree on an offset before it is trusted.</summary>
    private const int MIN_OUTLINE_CALIBRATION_ANCHORS = 2;

    private const int MIN_MARKDOWN_HEADING_LEVEL = 1;
    private const int MAX_MARKDOWN_HEADING_LEVEL = 6;

    private readonly record struct OutlineParagraphMatch(int PageIndex, int ParagraphIndex, int Depth);

    /// <summary>
    /// Give a paragraph the heading level its outline item implies. The item's depth is
    /// relative to the outline root, so it is shifted by an offset calibrated against the
    /// headings the classifier already found — and by a default when too few agree.
    /// </summary>
    private static void RecoverHeadingsFromOutline(
        List<List<PdfParagraph>> allPages, List<PdfOutlineEntry> outlineEntries)
    {
        var matches = CollectUniqueOutlineMatches(allPages, outlineEntries);
        int offset = CalibratedOutlineHeadingOffset(allPages, matches);

        foreach (var matched in matches)
        {
            var paragraph = allPages[matched.PageIndex][matched.ParagraphIndex];
            if (paragraph.HeadingLevel.HasValue || !OutlineLayoutAllowsHeading(paragraph)) continue;
            int level = Math.Clamp(matched.Depth + offset, MIN_MARKDOWN_HEADING_LEVEL, MAX_MARKDOWN_HEADING_LEVEL);
            paragraph.HeadingLevel = (byte)level;
            paragraph.IsListItem = false;
            paragraph.IsPageFurniture = false;
        }
    }

    /// <summary>
    /// Match outline items to paragraphs, keeping only the pairs where the title occurs
    /// exactly once in the outline for that page and exactly once among the page's
    /// paragraphs — an ambiguous title says nothing about which paragraph is the heading.
    /// </summary>
    private static List<OutlineParagraphMatch> CollectUniqueOutlineMatches(
        List<List<PdfParagraph>> allPages, List<PdfOutlineEntry> outlineEntries)
    {
        var outlineCounts = new Dictionary<(int, string), int>();
        foreach (var entry in outlineEntries)
        {
            var key = OutlineMatchKey(entry, allPages.Count);
            if (key is null) continue;
            outlineCounts[key.Value] = outlineCounts.GetValueOrDefault(key.Value) + 1;
        }

        var paragraphMatches = new List<Dictionary<string, (int Count, int Index)>>(allPages.Count);
        foreach (var page in allPages)
        {
            var map = new Dictionary<string, (int Count, int Index)>(StringComparer.Ordinal);
            for (int index = 0; index < page.Count; index++)
            {
                string title = NormalizeOutlineTitle(ParagraphTextRaw(page[index]));
                if (map.TryGetValue(title, out var existing)) map[title] = (existing.Count + 1, existing.Index);
                else map[title] = (1, index);
            }
            paragraphMatches.Add(map);
        }

        var result = new List<OutlineParagraphMatch>();
        foreach (var entry in outlineEntries)
        {
            var key = OutlineMatchKey(entry, allPages.Count);
            if (key is not { } k) continue;
            if (outlineCounts.GetValueOrDefault(k) != 1) continue;
            if (!paragraphMatches[k.Item1].TryGetValue(k.Item2, out var match)) continue;
            if (match.Count != 1) continue;
            result.Add(new OutlineParagraphMatch(k.Item1, match.Index, entry.Depth));
        }
        return result;
    }

    private static (int, string)? OutlineMatchKey(PdfOutlineEntry entry, int pageCount)
    {
        if (entry.PageNumber is not { } pageNumber || pageNumber < 1) return null;
        int pageIndex = pageNumber - 1;
        string title = NormalizeOutlineTitle(entry.Title);
        return pageIndex < pageCount && title.Length > 0 ? (pageIndex, title) : null;
    }

    /// <summary>
    /// The modal (level − depth) among matches the classifier already levelled. Only a
    /// single winner backed by enough anchors is trusted; anything else takes the default.
    /// </summary>
    private static int CalibratedOutlineHeadingOffset(
        List<List<PdfParagraph>> allPages, List<OutlineParagraphMatch> matches)
    {
        var counts = new Dictionary<int, int>();
        foreach (var matched in matches)
        {
            var paragraph = allPages[matched.PageIndex][matched.ParagraphIndex];
            if (!OutlineLayoutAllowsHeading(paragraph)) continue;
            if (paragraph.HeadingLevel is not { } level) continue;
            int delta = level - matched.Depth;
            counts[delta] = counts.GetValueOrDefault(delta) + 1;
        }

        if (counts.Count == 0) return DEFAULT_OUTLINE_HEADING_OFFSET;
        int maxCount = counts.Values.Max();
        var winners = counts.Where(kv => kv.Value == maxCount).ToList();
        return winners.Count == 1 && maxCount >= MIN_OUTLINE_CALIBRATION_ANCHORS
            ? winners[0].Key
            : DEFAULT_OUTLINE_HEADING_OFFSET;
    }

    private static bool OutlineLayoutAllowsHeading(PdfParagraph paragraph) =>
        !paragraph.IsCodeBlock && !paragraph.IsFormula;

    private static string ParagraphTextRaw(PdfParagraph p) =>
        p.Text.Length > 0 ? p.Text : ParagraphPlainText(p);

    /// <summary>
    /// Fold a title down to the letters and digits it is made of, so an outline item and
    /// the line it points at compare equal across section labels, punctuation and case.
    /// </summary>
    private static string NormalizeOutlineTitle(string text)
    {
        string stripped = StripSectionLabel(text.Trim());
        var normalized = new System.Text.StringBuilder();
        bool pendingSpace = false;
        foreach (char raw in stripped)
        {
            foreach (char character in char.ToLowerInvariant(raw).ToString())
            {
                if (char.IsLetterOrDigit(character))
                {
                    if (pendingSpace && normalized.Length > 0) normalized.Append(' ');
                    normalized.Append(character);
                    pendingSpace = false;
                }
                else if (normalized.Length > 0) pendingSpace = true;
            }
        }
        return normalized.ToString();
    }

    /// <summary>Drop a leading section label ("3.", "(iv)", "A:") from a title.</summary>
    private static string StripSectionLabel(string text)
    {
        int space = -1;
        for (int i = 0; i < text.Length; i++) if (char.IsWhiteSpace(text[i])) { space = i; break; }
        if (space < 0) return text;
        string first = text.Substring(0, space);
        string rest = text.Substring(space).TrimStart();
        if (first.Length == 0) return text;

        bool punctuated = first[0] is '(' or '['
            || first[^1] is '.' or ')' or ']' or ':';
        string core = first.Trim('(', '[', '.', ')', ']', ':');
        var decimalParts = core.Split('.');
        bool isDecimal = decimalParts.Length > 0
            && decimalParts.All(part => part.Length > 0 && part.All(char.IsAsciiDigit));
        bool decimalLabel = isDecimal && (punctuated || decimalParts.Length > 1 || core.Length <= 3);
        bool romanLabel = punctuated && core.Length > 0
            && core.All(c => char.ToUpperInvariant(c) is 'I' or 'V' or 'X' or 'L' or 'C' or 'D' or 'M');
        bool letterLabel = punctuated && core.Length == 1 && char.IsAsciiLetter(core[0]);

        return decimalLabel || romanLabel || letterLabel ? rest : text;
    }

    // ── heading refinement (classify.rs) ─────────────────────────────────────────

    internal static string ParagraphPlainText(PdfParagraph p) =>
        string.Join(" ", p.Lines.SelectMany(l => l.Segments).Select(s => s.Text));

    private static string EffectiveText(PdfParagraph p) => p.Text.Length > 0 ? p.Text : ParagraphPlainText(p);

    /// <summary>Words the lead clause must have before a colon can introduce a run-in list.</summary>
    private const int RUN_IN_LIST_MIN_LEAD_WORDS = 4;
    /// <summary>Clauses a run-in list must have; one semicolon is punctuation, not a list.</summary>
    private const int RUN_IN_LIST_MIN_ITEMS = 2;
    /// <summary>Words a single clause must have to read as an item rather than an aside.</summary>
    private const int RUN_IN_LIST_MIN_ITEM_WORDS = 3;

    /// <summary>
    /// Split "…the following: to exclude x; where y applies; unless z." into a lead paragraph
    /// and one list item per clause.
    /// </summary>
    /// <remarks>
    /// A run-in list is a list that the typesetter set as prose. Nothing in the geometry marks
    /// it — there are no bullets and no line breaks — so the only evidence is the colon and the
    /// semicolon-delimited clauses after it, and the gates are deliberately tight: a lead of
    /// several words, at least two clauses, and every clause substantial, lowercase-initial and
    /// clause-terminated. A capitalized clause is a new sentence, not an item.
    /// </remarks>
    private static void SplitColonSemicolonRunInLists(List<List<PdfParagraph>> allPageParagraphs)
    {
        foreach (var page in allPageParagraphs)
        {
            int index = 0;
            while (index < page.Count)
            {
                var replacement = TrySplitRunInList(page[index]);
                if (replacement is null) { index++; continue; }
                page.RemoveAt(index);
                page.InsertRange(index, replacement);
                index += replacement.Count;
            }
        }
    }

    private static List<PdfParagraph>? TrySplitRunInList(PdfParagraph para)
    {
        if (para.HeadingLevel.HasValue || para.IsListItem || para.IsCodeBlock
            || para.IsFormula || para.IsPageFurniture)
            return null;

        string normalized = string.Join(" ", EffectiveText(para)
            .Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries));
        int colon = normalized.LastIndexOf(':');
        if (colon < 0) return null;

        string lead = normalized.Substring(0, colon + 1).Trim();
        if (WordCount(lead) < RUN_IN_LIST_MIN_LEAD_WORDS) return null;

        string tail = normalized.Substring(colon + 1).TrimStart();
        if (tail.Length == 0) return null;

        var items = SplitInclusive(tail, ';')
            .Select(seg => seg.Trim())
            .Where(seg => seg.Length > 0)
            .ToList();
        if (items.Count < RUN_IN_LIST_MIN_ITEMS || !items.All(IsProbableRunInListItem)) return null;

        var split = new List<PdfParagraph>(items.Count + 1) { RunInListFragment(para, lead, false) };
        foreach (var item in items) split.Add(RunInListFragment(para, item, true));
        return split;
    }

    /// <summary>Split on <paramref name="sep"/>, keeping it at the end of each piece.</summary>
    private static List<string> SplitInclusive(string text, char sep)
    {
        var parts = new List<string>();
        int start = 0;
        for (int i = 0; i < text.Length; i++)
            if (text[i] == sep) { parts.Add(text.Substring(start, i - start + 1)); start = i + 1; }
        if (start < text.Length) parts.Add(text.Substring(start));
        return parts;
    }

    /// <summary>
    /// Whether one semicolon-delimited clause reads as a genuine list item: substantial, a
    /// lowercase continuation of the lead sentence rather than a new capitalized one, and
    /// clause-terminated.
    /// </summary>
    private static bool IsProbableRunInListItem(string item)
    {
        if (WordCount(item) < RUN_IN_LIST_MIN_ITEM_WORDS) return false;
        if (item.Length == 0 || !char.IsLower(item[0])) return false;
        return item[^1] is ';' or '.';
    }

    /// <summary>One split-off fragment, inheriting the source's non-textual attributes.</summary>
    private static PdfParagraph RunInListFragment(PdfParagraph source, string text, bool isListItem) => new()
    {
        Text = text,
        Lines = new(),
        DominantFontSize = source.DominantFontSize,
        HeadingLevel = null,
        IsBold = source.IsBold,
        IsListItem = isListItem,
        IsCodeBlock = false,
        IsFormula = false,
        IsPageFurniture = source.IsPageFurniture,
        BlockBbox = source.BlockBbox,
        WordCount = WordCount(text),
    };

    /// <summary>
    /// Close a one-level hole in the heading hierarchy: a document with exactly one H1, no H2 at
    /// all and something at H3 or below has skipped a level, so everything below H2 moves up one.
    /// </summary>
    /// <remarks>
    /// Only that exact shape qualifies. A document with several H1s has no single title for the
    /// deeper levels to hang from, and one that already uses H2 has no hole to close.
    /// </remarks>
    private static void CompactFinalHeadingHierarchy(List<List<PdfParagraph>> allPages)
    {
        int h1Count = 0;
        bool hasH2 = false, hasDeeper = false;
        foreach (var para in allPages.SelectMany(p => p))
        {
            if (para.HeadingLevel is not byte level) continue;
            if (level == 1) h1Count++;
            if (level == 2) hasH2 = true;
            if (level >= 3) hasDeeper = true;
        }
        if (h1Count != 1 || hasH2 || !hasDeeper) return;

        foreach (var para in allPages.SelectMany(p => p))
            if (para.HeadingLevel is byte level && level >= 3) para.HeadingLevel = (byte)(level - 1);
    }

    private static void RefineHeadingHierarchy(List<List<PdfParagraph>> allPages)
    {
        int H1Count() => allPages.SelectMany(p => p).Count(p => p.HeadingLevel == 1);

        if (H1Count() == 0)
        {
            bool hasAny = allPages.SelectMany(p => p).Any(p => p.HeadingLevel.HasValue);
            if (hasAny && !HasRepeatedSparsePeerHeadingTier(allPages)) PromoteTitleHeading(allPages);

            bool stillNoH1 = !allPages.SelectMany(p => p).Any(p => p.HeadingLevel == 1);
            if (stillNoH1 && allPages.Count > 0 && allPages[0].Count > 0)
            {
                var page0 = allPages[0];
                int totalParagraphs = allPages.Sum(p => p.Count);
                float maxFont = page0.Max(p => p.DominantFontSize);
                var firstP = page0[0];
                string firstText = ParagraphPlainText(firstP);
                int firstWc = WordCount(firstText);

                // The first paragraph must also stand out from the rest of the document, not
                // merely be the largest thing on its own page.
                float? restFont = OtherParagraphsFontSize(allPages, 0, 0);
                bool clearsFontGate = restFont is not { } bodyFont
                    || bodyFont <= 0f
                    || (firstP.DominantFontSize >= bodyFont * MIN_HEADING_FONT_RATIO
                        && firstP.DominantFontSize >= bodyFont + MIN_HEADING_FONT_GAP);

                if (totalParagraphs >= MIN_BLOCKS_FOR_FONT_HEADING
                    && clearsFontGate
                    && firstP.DominantFontSize >= maxFont && firstWc <= 10 && firstWc > 0
                    && !firstP.IsPageFurniture && !LooksLikeBareUrl(firstText))
                    page0[0].HeadingLevel = 1;
            }
        }

        if (H1Count() <= 1) return;
        foreach (var page in allPages) MergeConsecutiveH1s(page);
        if (H1Count() <= 1) return;

        var firstH1 = allPages.SelectMany(p => p).FirstOrDefault(p => p.HeadingLevel == 1);
        bool firstH1IsTitle = firstH1 != null && !StartsWithSectionNumber(ParagraphPlainText(firstH1));
        if (!firstH1IsTitle) return;

        bool foundFirst = false;
        foreach (var page in allPages)
            foreach (var para in page)
                if (para.HeadingLevel == 1)
                {
                    if (!foundFirst) { foundFirst = true; continue; }
                    if (StartsWithSectionNumber(ParagraphPlainText(para))) para.HeadingLevel = 2;
                }
    }

    /// <summary>
    /// True when a sparse document repeats the same H2 tier at the start of several pages.
    /// </summary>
    /// <remarks>
    /// Those openings are peer sections. Promoting the first of them to H1 would say the
    /// document has a title and that this is it, which is exactly what the repetition denies —
    /// so title inference is skipped and they stay level peers.
    /// </remarks>
    private static bool HasRepeatedSparsePeerHeadingTier(List<List<PdfParagraph>> allPages)
    {
        int paragraphCount = allPages.Sum(p => p.Count);
        if (paragraphCount >= MIN_BLOCKS_FOR_FONT_HEADING) return false;

        var leadingH2Fonts = new List<float>();
        foreach (var page in allPages)
        {
            var first = page.FirstOrDefault(p =>
                !p.IsPageFurniture && ParagraphPlainText(p).Trim().Length > 0);
            if (first is null) continue;
            if (first.HeadingLevel == 2 && !float.IsNaN(first.DominantFontSize)
                && !float.IsInfinity(first.DominantFontSize))
                leadingH2Fonts.Add(first.DominantFontSize);
        }

        return leadingH2Fonts.Any(candidate =>
            leadingH2Fonts.Count(f => Math.Abs(f - candidate) <= SPARSE_PEER_HEADING_FONT_TOLERANCE)
                >= SPARSE_PEER_HEADING_MIN_PAGES);
    }

    /// <summary>Character-weighted mean font size of every paragraph except the excluded one —
    /// the document's body size, used to judge whether the first paragraph really stands out.</summary>
    private static float? OtherParagraphsFontSize(List<List<PdfParagraph>> allPages, int excludePage, int excludeIndex)
    {
        double weightedSum = 0;
        long totalChars = 0;
        for (int pageIdx = 0; pageIdx < allPages.Count; pageIdx++)
        {
            Cancel.ThrowIfRequested();
            var page = allPages[pageIdx];
            for (int paraIdx = 0; paraIdx < page.Count; paraIdx++)
            {
                if (pageIdx == excludePage && paraIdx == excludeIndex) continue;
                int charCount = ParagraphPlainText(page[paraIdx]).Length;
                if (charCount == 0) continue;
                weightedSum += (double)page[paraIdx].DominantFontSize * charCount;
                totalChars += charCount;
            }
        }
        return totalChars == 0 ? null : (float)(weightedSum / totalChars);
    }

    private static void PromoteTitleHeading(List<List<PdfParagraph>> allPages)
    {
        if (allPages.Count == 0) return;
        var page = allPages[0];
        var headings = new List<(int idx, float fs)>();
        for (int i = 0; i < page.Count; i++) if (page[i].HeadingLevel.HasValue) headings.Add((i, page[i].DominantFontSize));
        if (headings.Count == 0) return;
        if (headings.Count == 1) { page[headings[0].idx].HeadingLevel = 1; return; }
        float maxSize = headings.Max(h => h.fs);
        float secondMax = headings.Where(h => h.fs < maxSize).Select(h => h.fs).DefaultIfEmpty(0f).Max();
        if (maxSize - secondMax >= 1.5f)
        {
            var idx = headings.First(h => h.fs == maxSize).idx;
            page[idx].HeadingLevel = 1;
        }
    }

    private static void MergeConsecutiveH1s(List<PdfParagraph> page)
    {
        int i = 0;
        while (i < page.Count)
        {
            if (page[i].HeadingLevel != 1) { i++; continue; }
            float baseFs = page[i].DominantFontSize;
            int runEnd = i + 1;
            while (runEnd < page.Count && page[runEnd].HeadingLevel == 1
                && Math.Abs(page[runEnd].DominantFontSize - baseFs) < 0.5f
                && LooksLikeTitleContinuation(page[runEnd - 1], page[runEnd]))
                runEnd++;
            if (runEnd - i > 1)
            {
                var mergedLines = page[i].Lines;
                var textParts = new List<string>();
                if (page[i].Text.Length > 0) textParts.Add(page[i].Text);
                for (int j = i + 1; j < runEnd; j++)
                {
                    mergedLines.AddRange(page[j].Lines);
                    if (page[j].Text.Length > 0) textParts.Add(page[j].Text);
                }
                page[i].Lines = mergedLines;
                if (textParts.Count > 0) page[i].Text = string.Join(" ", textParts);
                page.RemoveRange(i + 1, runEnd - (i + 1));
            }
            i++;
        }
    }

    private static bool LooksLikeTitleContinuation(PdfParagraph prev, PdfParagraph next)
    {
        string nextText = EffectiveText(next);
        if (LooksLikeStandaloneHeadingText(nextText)) return false;
        string prevText = EffectiveText(prev);
        string pt = prevText.TrimEnd();
        if (pt.Length > 0 && (pt[^1] is '.' or '!' or '?' or ':')) return false;
        return WordCount(prevText) <= 4 && WordCount(nextText) <= 4;
    }

    private static bool LooksLikeStandaloneHeadingText(string text)
    {
        string trimmed = text.Trim();
        var words = trimmed.Split(' ');
        string firstW = words.Length > 0 ? words[0] : "";
        string secondW = words.Length > 1 ? words[1] : "";
        if (firstW.Length > 0 && char.IsAsciiDigit(firstW[0])) return true;
        bool fAlpha = firstW.Any(c => char.IsAsciiLetter(c));
        bool fDigit = firstW.Any(c => char.IsAsciiDigit(c));
        if (fAlpha && fDigit) return true;
        if (secondW.Length > 0 && firstW.All(c => char.IsAsciiLetter(c)) && char.IsAsciiDigit(secondW[0])) return true;
        return false;
    }

    private static void DemoteUnnumberedSubsections(List<List<PdfParagraph>> allPages)
    {
        var h2Info = new List<(int page, int para, bool numbered)>();
        for (int pi = 0; pi < allPages.Count; pi++)
            for (int qi = 0; qi < allPages[pi].Count; qi++)
                if (allPages[pi][qi].HeadingLevel == 2)
                    h2Info.Add((pi, qi, StartsWithSectionNumber(ParagraphPlainText(allPages[pi][qi]))));

        int numberedCount = h2Info.Count(x => x.numbered);
        if (numberedCount < 3) return;

        var numberedPos = new List<int>();
        for (int i = 0; i < h2Info.Count; i++) if (h2Info[i].numbered) numberedPos.Add(i);

        for (int w = 0; w + 1 < numberedPos.Count; w++)
        {
            int start = numberedPos[w], end = numberedPos[w + 1];
            for (int k = start + 1; k < end; k++)
            {
                var (pg, pa, isNum) = h2Info[k];
                if (!isNum) allPages[pg][pa].HeadingLevel = 3;
            }
        }
    }

    private static void DemoteHeadingRuns(List<List<PdfParagraph>> allPages)
    {
        const int MAX_CONSECUTIVE = 3;
        foreach (var page in allPages)
        {
            int runStart = 0;
            while (runStart < page.Count)
            {
                if (page[runStart].HeadingLevel is not byte level) { runStart++; continue; }
                int runEnd = runStart + 1;
                while (runEnd < page.Count && page[runEnd].HeadingLevel == level) runEnd++;
                if (runEnd - runStart > MAX_CONSECUTIVE)
                    for (int k = runStart + 1; k < runEnd; k++) page[k].HeadingLevel = null;
                runStart = runEnd;
            }
        }
    }

    // ── repeating text / arxiv noise (classify.rs) ───────────────────────────────

    private static void MarkCrossPageRepeatingText(List<List<PdfParagraph>> allPages, float[] pageHeights)
    {
        if (allPages.Count < 4) return;
        const float marginFrac = 0.10f;

        var textPageCount = new Dictionary<string, int>();
        var alphanumToExact = new Dictionary<string, HashSet<string>>();
        var firstSeen = new Dictionary<string, int>();

        for (int pi = 0; pi < allPages.Count; pi++)
        {
            float ph = pi < pageHeights.Length ? pageHeights[pi] : 792.0f;
            float topMargin = ph * (1f - marginFrac);
            float bottomMargin = ph * marginFrac;
            var seen = new HashSet<string>();
            foreach (var para in allPages[pi])
            {
                if (para.IsPageFurniture) continue;
                bool inMargin = para.BlockBbox is { } bb && (bb.T > topMargin || bb.B < bottomMargin);
                if (!inMargin) continue;
                string norm = ParagraphPlainText(para).Trim().ToLowerInvariant();
                if (norm.Length == 0) continue;
                string key = new string(norm.Where(char.IsLetterOrDigit).ToArray());
                if (key.Length == 0) continue;
                if (!alphanumToExact.TryGetValue(key, out var set)) alphanumToExact[key] = set = new();
                set.Add(norm);
                if (seen.Add(key))
                {
                    if (!textPageCount.TryGetValue(key, out int c) || c == 0) firstSeen[key] = pi;
                    textPageCount[key] = c + 1;
                }
            }
        }

        int threshold = allPages.Count / 2;
        var repeating = new HashSet<string>();
        foreach (var (key, count) in textPageCount)
            if (count > threshold && alphanumToExact.TryGetValue(key, out var variants))
                foreach (var v in variants) repeating.Add(v);
        if (repeating.Count == 0) return;

        for (int pi = 0; pi < allPages.Count; pi++)
        {
            float ph = pi < pageHeights.Length ? pageHeights[pi] : 792.0f;
            float topMargin = ph * (1f - marginFrac);
            float bottomMargin = ph * marginFrac;
            foreach (var para in allPages[pi])
            {
                if (para.IsPageFurniture) continue;
                bool inMargin = para.BlockBbox is { } bb && (bb.T > topMargin || bb.B < bottomMargin);
                if (!inMargin) continue;
                string norm = ParagraphPlainText(para).Trim().ToLowerInvariant();
                if (repeating.Contains(norm))
                {
                    string key = new string(norm.Where(char.IsLetterOrDigit).ToArray());
                    if (firstSeen.TryGetValue(key, out int fp) && fp == pi) continue;
                    para.IsPageFurniture = true;
                    para.HeadingLevel = null;
                }
            }
        }
    }

    private static void MarkCrossPageRepeatingShortText(List<List<PdfParagraph>> allPages)
    {
        if (allPages.Count < 5) return;
        const int maxWords = 20;
        int threshold = (int)Math.Ceiling(allPages.Count * 0.7);

        var textPageCount = new Dictionary<string, int>();
        var firstSeen = new Dictionary<string, int>();
        for (int pi = 0; pi < allPages.Count; pi++)
        {
            var seen = new HashSet<string>();
            foreach (var para in allPages[pi])
            {
                if (para.IsPageFurniture) continue;
                string norm = ParagraphPlainText(para).Trim().ToLowerInvariant();
                if (norm.Length == 0) continue;
                if (WordCount(norm) > maxWords) continue;
                string key = new string(norm.Where(char.IsLetterOrDigit).ToArray());
                if (key.Length == 0) continue;
                if (seen.Add(key))
                {
                    if (!textPageCount.TryGetValue(key, out int c) || c == 0) firstSeen[key] = pi;
                    textPageCount[key] = c + 1;
                }
            }
        }

        var repeating = new HashSet<string>();
        foreach (var (key, count) in textPageCount) if (count >= threshold) repeating.Add(key);
        if (repeating.Count == 0) return;

        for (int pi = 0; pi < allPages.Count; pi++)
            foreach (var para in allPages[pi])
            {
                if (para.IsPageFurniture) continue;
                string norm = ParagraphPlainText(para).Trim().ToLowerInvariant();
                if (WordCount(norm) > maxWords) continue;
                string key = new string(norm.Where(char.IsLetterOrDigit).ToArray());
                if (key.Length == 0) continue;
                if (repeating.Contains(key))
                {
                    if (firstSeen.TryGetValue(key, out int fp) && fp == pi) continue;
                    para.IsPageFurniture = true;
                    para.HeadingLevel = null;
                }
            }
    }

    private static readonly Regex ArxivRe = new(@"arXiv:\d{4}\.\d{4,5}", RegexOptions.Compiled);
    private static readonly Regex ArxivTrailingRe = new(
        @"(?:\s+(?:\S+\s+){0,8})?arXiv:\d{4}\.\d{4,5}(?:v\d+)?(?:\s*\[[\w.-]+\])?\s*(?:\d{1,2}\s+\w+\s+\d{4})?\s*$",
        RegexOptions.Compiled);

    private static void MarkArxivNoise(List<List<PdfParagraph>> allPages)
    {
        for (int pi = 0; pi < Math.Min(2, allPages.Count); pi++)
            foreach (var para in allPages[pi])
            {
                if (para.IsPageFurniture) continue;
                string trimmed = ParagraphPlainText(para).Trim();
                int wc = WordCount(trimmed);
                if (!ArxivRe.IsMatch(trimmed)) continue;
                if (wc <= 25) { para.IsPageFurniture = true; para.HeadingLevel = null; }
                else
                {
                    var m = ArxivTrailingRe.Match(trimmed);
                    if (m.Success) StripTrailingTextFromParagraph(para, trimmed.Substring(m.Index).Trim());
                }
            }
    }

    private static void StripTrailingTextFromParagraph(PdfParagraph para, string noise)
    {
        for (int li = para.Lines.Count - 1; li >= 0; li--)
        {
            var line = para.Lines[li];
            for (int si = line.Segments.Count - 1; si >= 0; si--)
            {
                var seg = line.Segments[si];
                int pos = seg.Text.IndexOf(noise, StringComparison.Ordinal);
                if (pos >= 0) { seg.Text = seg.Text.Substring(0, pos).TrimEnd(); return; }
                string segTrimmed = seg.Text.Trim();
                if (segTrimmed.Length > 0 && noise.Contains(segTrimmed, StringComparison.Ordinal)) seg.Text = "";
                else return;
            }
        }
    }

    private static void DeduplicateParagraphs(List<List<PdfParagraph>> allPages)
    {
        foreach (var page in allPages)
        {
            if (page.Count < 2) continue;
            int i = 0;
            while (i + 1 < page.Count)
            {
                string a = ParagraphTextNormalized(page[i]);
                string b = ParagraphTextNormalized(page[i + 1]);
                if (a.Length >= 5 && a == b) page.RemoveAt(i + 1);
                else i++;
            }
            var seen = new HashSet<string>();
            var toRemove = new List<int>();
            for (int idx = 0; idx < page.Count; idx++)
            {
                if (!IsDedupCandidate(page[idx])) continue;
                string text = ParagraphTextNormalized(page[idx]);
                if (text.Length < 15) continue;
                if (!seen.Add(text)) toRemove.Add(idx);
            }
            for (int r = toRemove.Count - 1; r >= 0; r--) page.RemoveAt(toRemove[r]);
        }
    }

    private static string ParagraphTextNormalized(PdfParagraph p)
    {
        string raw = p.Text.Length == 0 ? ParagraphPlainText(p) : p.Text;
        return string.Join(" ", raw.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries)).ToLowerInvariant();
    }

    private static bool IsDedupCandidate(PdfParagraph p) =>
        p.HeadingLevel is null && !p.IsListItem && !p.IsCodeBlock && !p.IsFormula && !p.IsPageFurniture;

    // ── predicates (classify.rs / pipeline.rs / layout_classify.rs / regions) ────

    private static bool IsBareListMarker(string text)
    {
        string t = text.Trim();
        if (t.Length == 0 || t.Length > 5) return false;
        if (t is "•" or "·" or "◦" or "▪" or "–" or "—" or "-" or "*") return true;
        if (t.StartsWith('(') && t.EndsWith(')'))
        {
            string inner = t.Substring(1, t.Length - 2);
            return inner.Length > 0 && inner.All(char.IsLetterOrDigit);
        }
        if (t.EndsWith('.') || t.EndsWith(')'))
        {
            string body = t.Substring(0, t.Length - 1);
            return body.Length > 0 && body.Length <= 2 && body.All(char.IsLetterOrDigit);
        }
        return false;
    }

    /// <summary>
    /// Whether a line opens with a list marker. Ports <c>looks_like_list_item</c>: a bullet
    /// glyph, a dash followed by a space, or an ordered marker that has both a separator and
    /// alphabetic content after it.
    /// </summary>
    internal static bool LooksLikeListItem(string text)
    {
        string t = text.TrimStart();
        if (t.Length == 0) return false;

        if (t[0] is '\u2022' or '\u00b7' or '\u25e6' or '\u25aa') return true;

        if (t[0] is '\u2013' or '\u2014')
        {
            string rest = t[1..];
            if (!rest.StartsWith(' ') && !rest.StartsWith('\t')) return false;
            string body = rest.TrimStart(' ', '\t');
            return body.Length != 0 && body[0] is not ('\r' or '\n');
        }

        if (t.StartsWith("- "))
        {
            string rest = t[2..];
            return rest.Length != 0 && char.IsLetter(rest[0]);
        }

        if (IsNumberedSectionHeading(t)) return false;

        if (PdfListMarker.Parse(t) is not { } marker) return false;
        if (marker.ContentStart >= t.Length) return false;
        char firstContentChar = t[marker.ContentStart];
        return marker.HasContent
            && marker.HasSeparator
            && !PdfListMarker.IsProbableAuthorByline(t)
            && char.IsLetter(firstContentChar)
            && !IsInlineParenthesizedQuantity(t, marker, firstContentChar);
    }

    /// <summary>
    /// Whether a line-leading <c>(N)</c> reads as a mid-sentence quantity clarification rather
    /// than a list marker.
    /// </summary>
    /// <remarks>
    /// "Two" followed by a line break and "(2) additional on-street parking spaces" wraps onto a
    /// physical line that
    /// <em>starts</em> with <c>(2)</c>, which is shaped identically to a genuine numbered marker
    /// like <c>(2) Second item</c>, so the sentence was split into a list item mid-clause.
    /// <para>
    /// The distinguishing signal is capitalization plus how the marker was separated from its
    /// content. A newline between the two means the marker arrived as
    /// its own text run, glued to the next by line reconstruction rather than by the author — that
    /// shape is trusted regardless of case. A plain space on the same physical line followed by a
    /// lowercase word is the shape of a number spelled out in prose; a genuine enumerated item is
    /// a new sentence and starts with a capital.
    /// </para>
    /// <para>
    /// Scoped to parenthesized <em>numeric</em> markers only: <c>(a)</c>/<c>(b)</c> are genuine
    /// sub-item markers (nobody writes "two (b) items"), and non-parenthesized families
    /// (<c>"1. "</c>, <c>"[1] "</c>) have no English idiom that produces this false positive.
    /// </para>
    /// </remarks>
    private static bool IsInlineParenthesizedQuantity(
        string t, PdfListMarker.OrderedListMarker marker, char firstContentChar)
    {
        if (!t.StartsWith('(') || marker.NumericValue is null) return false;
        string separatorRegion = t[..Math.Min(marker.ContentStart, t.Length)];
        if (separatorRegion.Contains('\n') || separatorRegion.Contains('\r')) return false;
        return !char.IsUpper(firstContentChar);
    }

    private static bool IsStructuralHeadingWord(string text) => text.Trim() switch
    {
        "Abstract" or "References" or "Appendix" or "Acknowledgments" or "Acknowledgements"
        or "Conclusion" or "Conclusions" or "Bibliography" or "Contents" or "Index"
        or "Glossary" or "Summary" or "Discussion" or "Methods" or "Results" or "Methodology" => true,
        _ => false,
    };

    internal static bool IsSeparatorText(string text)
    {
        string trimmed = text.Trim();
        if (trimmed.Length == 0) return false;
        int total = trimmed.Length;
        int alnum = trimmed.Count(char.IsLetterOrDigit);
        if (alnum == 0) return true;
        return total >= 6 && ((double)alnum / total) < 0.15;
    }

    internal static bool LooksLikeFigureLabel(string text)
    {
        var words = text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
        if (words.Length >= 3 && words.All(w => w.Length <= 1)) return true;
        if (words.Length >= 5)
            foreach (var w in words)
            {
                string lw = w.ToLowerInvariant();
                if (words.Count(x => x.ToLowerInvariant() == lw) >= 3) return true;
            }
        return false;
    }

    internal static bool LooksLikeBareUrl(string text)
    {
        string t = text.Trim();
        return (t.StartsWith("http://") || t.StartsWith("https://") || t.StartsWith("www.")) && !t.Any(char.IsWhiteSpace);
    }

    internal static bool IsSectionPattern(string text)
    {
        string t = text.Trim();
        if (t.StartsWith('§')) return true;
        int words = WordCount(t);
        if (words <= 6 && t.Where(char.IsLetter).All(char.IsUpper)) return true;
        return StartsWithSectionNumber(t);
    }

    internal static bool IsNumberedSectionHeading(string text)
    {
        string t = text.Trim();
        if (t.Length == 0) return false;
        // Roman-numeral markers.
        int romanEnd = 0;
        while (romanEnd < t.Length && "IVXLCDM".IndexOf(t[romanEnd]) >= 0) romanEnd++;
        if (romanEnd > 0 && romanEnd < t.Length && (t[romanEnd] is '.' or ' ' or ')') && IsValidRoman(t.Substring(0, romanEnd)))
            return true;

        int levels = 0, idx = 0;
        while (true)
        {
            int digitLen = 0;
            while (idx + digitLen < t.Length && char.IsAsciiDigit(t[idx + digitLen])) digitLen++;
            if (digitLen == 0) break;
            levels++;
            idx += digitLen;
            if (idx < t.Length && t[idx] == '.' && idx + 1 < t.Length && char.IsAsciiDigit(t[idx + 1])) idx++;
            else break;
        }
        if (levels == 0) return false;
        if (levels >= 2) return true;
        if (idx < t.Length && (t[idx] == '.' || t[idx] == ')')) idx++;
        string remainder = idx < t.Length ? t.Substring(idx).TrimStart() : "";
        return remainder.Any(char.IsLetter) && remainder.Where(char.IsLetter).All(char.IsUpper);
    }

    /// <summary>
    /// Infer a heading level for a bold/large paragraph. Section numbering wins; otherwise the
    /// level comes from how far the font rises above body text, and a document with no body
    /// baseline at all gets H2 rather than H1 (Rust <c>infer_bold_heading_level</c>).
    /// </summary>
    internal static byte InferBoldHeadingLevel(float fontSize, float bodyFontSize, string text)
    {
        if (StartsWithSectionNumber(text)) return InferSectionLevel(text);
        if (bodyFontSize > 0f) return fontSize / bodyFontSize > 1.2f ? (byte)2 : (byte)3;
        return 2;
    }

    /// <summary>
    /// Infer heading level from section numbering depth: "1 Introduction" / "I. INTRO" /
    /// "A. Proofs" are H2, "1.1 Details" is H3, "1.1.1 Deep" is H4
    /// (Rust <c>infer_section_level</c>).
    /// </summary>
    internal static byte InferSectionLevel(string text)
    {
        string trimmed = text.Trim();
        if (trimmed.Length == 0) return 2;

        char firstChar = trimmed[0];
        bool isAlphaPrefix = char.IsAsciiLetter(firstChar) && trimmed.Length >= 2
            && trimmed[1] is '.' or ')' or ' ';

        int numberingEnd;
        if (isAlphaPrefix)
        {
            string afterLetter = trimmed[1..];
            int restEnd = 0;
            while (restEnd < afterLetter.Length
                   && (char.IsAsciiDigit(afterLetter[restEnd]) || afterLetter[restEnd] == '.')) restEnd++;
            if (restEnd == afterLetter.Length) restEnd = 0;
            numberingEnd = 1 + restEnd;
        }
        else
        {
            int romanEnd = 0;
            while (romanEnd < trimmed.Length && "IVXLCDM".IndexOf(trimmed[romanEnd]) >= 0) romanEnd++;
            if (romanEnd > 0 && romanEnd <= 5 && romanEnd < trimmed.Length)
            {
                char next = trimmed[romanEnd];
                if ((next is '.' or ' ' or ')') && IsValidRoman(trimmed[..romanEnd])) return 2;
            }
            int end = 0;
            while (end < trimmed.Length && (char.IsAsciiDigit(trimmed[end]) || trimmed[end] == '.')) end++;
            numberingEnd = end == trimmed.Length ? 0 : end;
        }

        if (numberingEnd == 0) return 2;

        string numbering = trimmed[..numberingEnd];
        int dotCount = numbering.Count(c => c == '.');
        int effectiveDots = numbering.EndsWith('.') ? Math.Max(0, dotCount - 1) : dotCount;
        return effectiveDots switch { 0 => (byte)2, 1 => (byte)3, _ => (byte)4 };
    }

    private static bool StartsWithSectionNumber(string text)
    {
        string trimmed = text.Trim();
        if (trimmed.Length == 0) return false;
        int digitEnd = 0;
        while (digitEnd < trimmed.Length && char.IsAsciiDigit(trimmed[digitEnd])) digitEnd++;
        if (digitEnd > 0 && digitEnd < trimmed.Length)
        {
            char next = trimmed[digitEnd];
            if (next is ' ' or '.' or ')') return true;
        }
        int romanEnd = 0;
        while (romanEnd < trimmed.Length && "IVXLCDM".IndexOf(trimmed[romanEnd]) >= 0) romanEnd++;
        if (romanEnd > 0 && romanEnd <= 5 && romanEnd < trimmed.Length)
        {
            char next = trimmed[romanEnd];
            if ((next is '.' or ' ' or ')') && IsValidRoman(trimmed.Substring(0, romanEnd))) return true;
        }
        return false;
    }

    private static bool IsValidRoman(string s) => s switch
    {
        "I" or "II" or "III" or "IV" or "V" or "VI" or "VII" or "VIII" or "IX" or "X"
        or "XI" or "XII" or "XIII" or "XIV" or "XV" or "XVI" or "XVII" or "XVIII" or "XIX" or "XX" => true,
        _ => false,
    };

    // ── assembly (assembly.rs) ───────────────────────────────────────────────────

    /// <summary>
    /// Drop the segments a table already covers: a segment overlapping a table's box by at least
    /// half is that table's own text, and leaving it in the stream emits the grid twice.
    /// </summary>
    private static List<SegmentData> FilterSegmentsByTableBboxes(List<SegmentData> segments, List<Table> tables)
    {
        var boxes = tables.Select(t => t.BoundingBox).OfType<BoundingBox>().ToList();
        if (boxes.Count == 0) return segments;

        return segments.Where(seg =>
        {
            float area = seg.Width * seg.Height;
            if (area <= 0.0f || seg.Text.Trim().Length == 0) return true;
            return !boxes.Any(bb =>
            {
                float left = Math.Max(seg.X, (float)bb.X0);
                float right = Math.Min(seg.X + seg.Width, (float)bb.X1);
                float bottom = Math.Max(seg.Y, (float)bb.Y0);
                float top = Math.Min(seg.Y + seg.Height, (float)bb.Y1);
                if (left >= right || bottom >= top) return false;
                return (right - left) * (top - bottom) / area >= 0.5f;
            });
        }).ToList();
    }

    private static InternalDocument AssembleInternalDocument(
        List<List<PdfParagraph>> pages, List<List<Table>> tablesByPage)
    {
        var builder = new InternalDocumentBuilder("pdf");
        bool hasEmitted = false;
        for (int pageIdx = 0; pageIdx < pages.Count; pageIdx++)
        {
            Cancel.ThrowIfRequested();
            var paragraphs = pages[pageIdx];
            var pageTables = pageIdx < tablesByPage.Count
                ? tablesByPage[pageIdx].Where(t => t.Markdown.Trim().Length > 0).ToList()
                : new List<Table>();
            uint pageNum = (uint)(pageIdx + 1);
            bool pageHasContent = paragraphs.Count > 0 || pageTables.Count > 0;
            if (pageHasContent && hasEmitted) builder.PushPageBreak();
            if (pageTables.Count > 0) AssemblePageElementsWithTables(builder, paragraphs, pageTables, pageNum);
            else AssemblePageElements(builder, paragraphs, pageNum);
            if (pageHasContent) hasEmitted = true;
        }
        return builder.Build();
    }

    private static void AssemblePageElements(InternalDocumentBuilder builder, List<PdfParagraph> paragraphs, uint page)
    {
        bool inList = false;
        foreach (var para in paragraphs)
        {
            if (para.IsListItem && !inList) { builder.PushList(ListItemIsOrdered(para)); inList = true; }
            else if (!para.IsListItem && inList) { builder.EndList(); inList = false; }
            PushParagraphElement(builder, para, page);
        }
        if (inList) builder.EndList();
    }

    /// <summary>
    /// Emit a page whose paragraphs are interleaved with tables, each table placed at the reading
    /// -order boundary its top edge falls on.
    /// </summary>
    private static void AssemblePageElementsWithTables(
        InternalDocumentBuilder builder, List<PdfParagraph> paragraphs, List<Table> tables, uint page)
    {
        var positioned = new List<(float Top, Table Table)>();
        var unpositioned = new List<Table>();
        foreach (var table in tables)
        {
            if (table.BoundingBox is { } bb && float.IsFinite((float)bb.Y1)) positioned.Add(((float)bb.Y1, table));
            else unpositioned.Add(table);
        }
        positioned.Sort((a, b) => b.Top.CompareTo(a.Top));

        var tablesAtSlot = new List<Table>[paragraphs.Count + 1];
        for (int i = 0; i <= paragraphs.Count; i++) tablesAtSlot[i] = new List<Table>();
        foreach (var (top, table) in positioned)
            tablesAtSlot[TableInsertionSlot(paragraphs, table, top)].Add(table);

        bool inList = false;
        for (int slot = 0; slot <= paragraphs.Count; slot++)
        {
            foreach (var table in tablesAtSlot[slot])
            {
                if (inList) { builder.EndList(); inList = false; }
                PushTableElement(builder, table, page);
            }
            if (slot >= paragraphs.Count) break;

            var para = paragraphs[slot];
            if (para.IsListItem && !inList) { builder.PushList(ListItemIsOrdered(para)); inList = true; }
            else if (!para.IsListItem && inList) { builder.EndList(); inList = false; }
            PushParagraphElement(builder, para, page);
        }
        if (inList) builder.EndList();

        foreach (var table in unpositioned) PushTableElement(builder, table, page);
    }

    private static void PushTableElement(InternalDocumentBuilder builder, Table table, uint page) =>
        builder.PushTable(table, page, table.BoundingBox);

    /// <summary>
    /// The reading-order boundary a table belongs at: the first paragraph that starts below it.
    /// When every paragraph has horizontal geometry and the table overlaps only some of them,
    /// those identify the table's column and the others are ignored.
    /// </summary>
    private static int TableInsertionSlot(List<PdfParagraph> paragraphs, Table table, float tableY)
    {
        int Fallback() => VerticalInsertionSlot(
            Enumerable.Range(0, paragraphs.Count).Select(i => (i, paragraphs[i])), tableY, paragraphs.Count);

        if (table.BoundingBox is not { } bbox) return Fallback();

        var overlapping = new List<(int Slot, PdfParagraph Para)>();
        bool hasNonOverlapping = false;
        for (int slot = 0; slot < paragraphs.Count; slot++)
        {
            if (ParagraphHorizontalBounds(paragraphs[slot]) is not { } bounds) return Fallback();
            if (HorizontalRangesOverlap((float)bbox.X0, (float)bbox.X1, bounds.Left, bounds.Right))
                overlapping.Add((slot, paragraphs[slot]));
            else hasNonOverlapping = true;
        }

        if (overlapping.Count == 0 || !hasNonOverlapping) return Fallback();

        int endSlot = overlapping[^1].Slot + 1;
        return VerticalInsertionSlot(overlapping.Select(o => (o.Slot, o.Para)), tableY, endSlot);
    }

    private static int VerticalInsertionSlot(
        IEnumerable<(int Slot, PdfParagraph Para)> paragraphs, float tableY, int endSlot)
    {
        foreach (var (slot, para) in paragraphs)
            if (ParagraphVerticalAnchor(para) is { } anchor && anchor < tableY) return slot;
        return endSlot;
    }

    private static (float Left, float Right)? ParagraphHorizontalBounds(PdfParagraph paragraph)
    {
        if (paragraph.BlockBbox is { } bb && float.IsFinite(bb.L) && float.IsFinite(bb.R) && bb.R > bb.L)
            return (bb.L, bb.R);

        float left = float.PositiveInfinity, right = float.NegativeInfinity;
        foreach (var line in paragraph.Lines)
            foreach (var segment in line.Segments)
            {
                left = Math.Min(left, segment.X);
                right = Math.Max(right, segment.X + segment.Width);
            }
        return float.IsFinite(left) && float.IsFinite(right) && right > left ? (left, right) : null;
    }

    private static bool HorizontalRangesOverlap(float firstLeft, float firstRight, float secondLeft, float secondRight) =>
        float.IsFinite(firstLeft) && float.IsFinite(firstRight) && firstRight > firstLeft
        && firstLeft < secondRight && firstRight > secondLeft;

    private static float? ParagraphVerticalAnchor(PdfParagraph paragraph)
    {
        float? anchor = paragraph.BlockBbox is { } bb ? bb.T
            : paragraph.Lines.Count > 0 ? paragraph.Lines[0].BaselineY : null;
        return anchor is { } value && float.IsFinite(value) ? value : null;
    }

    private static BoundingBox? Bbox(PdfParagraph p) => p.BlockBbox is { } bb
        ? new BoundingBox { X0 = bb.L, Y0 = bb.B, X1 = bb.R, Y1 = bb.T } : null;

    private static string GetText(PdfParagraph para) =>
        FinalizeHyphens(para.Text.Length > 0 ? para.Text : JoinLineTextsPlain(para.Lines));

    private static void PushParagraphElement(InternalDocumentBuilder builder, PdfParagraph para, uint page)
    {
        var bbox = Bbox(para);
        if (para.HeadingLevel is byte level) { builder.PushHeading(level, GetText(para), page, bbox); return; }

        if (para.IsCodeBlock)
        {
            string text = para.Text.Length > 0 ? para.Text
                : string.Join("\n", para.Lines.Select(l => CollapseInnerSpaces(string.Join(" ", l.Segments.Select(s => s.Text)))));
            builder.PushCode(text, null, page, bbox);
            return;
        }

        if (para.IsFormula) { builder.PushFormula(GetText(para), page, bbox); return; }

        if (para.IsListItem)
        {
            string text = GetText(para);
            bool ordered = ListItemIsOrdered(para);
            List<TextAnnotation> anns;
            if (para.Text.Length == 0)
            {
                // Segment-derived annotations are byte offsets into the segment-derived text.
                // Where that text and the emitted text disagree — hyphen finalization moved a
                // byte — an offset into the wrong string is worse than no emphasis at all.
                var (annotatedText, segmentAnnotations) = ExtractTextAndAnnotations(para);
                anns = annotatedText == text ? segmentAnnotations : new List<TextAnnotation>();
            }
            else if (para.IsBold)
                anns = new() { new TextAnnotation { Start = 0, End = (uint)Utf8Len(text), Kind = AnnotationKind.Bold } };
            else anns = new();
            var (normalized, removedPrefixLen) = NormalizeListText(text);
            anns = ShiftAnnotationsAfterPrefixRemoval(anns, removedPrefixLen, Utf8Len(normalized));
            uint listItemIndex = builder.PushListItem(normalized, ordered, anns, page, bbox);
            // The marker just stripped off `text` is the document's own label. Keep it: an
            // ordinance is cross-referenced by printed clause number, so a synthesized sequence
            // position is not an acceptable substitute — "B." rendering as "1." renumbers the text.
            if (ordered && removedPrefixLen > 0)
                builder.SetListItemSourceLabel(listItemIndex, text[..removedPrefixLen].Trim());
            return;
        }

        if (para.IsPageFurniture)
        {
            string text = GetText(para);
            uint idx = builder.PushParagraph(text, new(), page, bbox);
            builder.SetLayer(idx, GuessFurnitureLayer(para));
            return;
        }

        // Default body paragraph.
        if (para.Text.Length > 0)
        {
            var anns = para.IsBold
                ? new List<TextAnnotation> { new TextAnnotation { Start = 0, End = (uint)Utf8Len(para.Text), Kind = AnnotationKind.Bold } }
                : new List<TextAnnotation>();
            builder.PushParagraph(FinalizeHyphens(para.Text), anns, page, bbox);
        }
        else
        {
            var (text, anns) = ExtractTextAndAnnotations(para);
            builder.PushParagraph(text, anns, page, bbox);
        }
    }

    private static ContentLayer GuessFurnitureLayer(PdfParagraph para)
    {
        // Page numbers etc.; header vs footer by vertical position is approximate.
        // Rust guess_furniture_layer defaults based on position; use Footer for low bbox, else Header.
        if (para.BlockBbox is { } bb) return bb.B < 200f ? ContentLayer.Footer : ContentLayer.Header;
        return ContentLayer.Header;
    }

    private static (string, List<TextAnnotation>) ExtractTextAndAnnotations(PdfParagraph para)
    {
        var all = para.Lines.SelectMany(l => l.Segments).ToList();
        if (all.Count == 0) return ("", new());
        var text = new StringBuilder();
        var annotations = new List<TextAnnotation>();
        int i = 0;
        while (i < all.Count)
        {
            bool bold = all[i].IsBold;
            bool italic = all[i].IsItalic;
            int runStart = i;
            while (i < all.Count && all[i].IsBold == bold && all[i].IsItalic == italic) i++;

            // Each word remembers the segment it came from: whether two words want a space
            // between them is a geometric question once they cross a segment boundary.
            var runWords = new List<(string Word, int Segment)>();
            for (int k = runStart; k < i; k++)
                foreach (var w in all[k].Text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
                    runWords.Add((w, k));

            if (text.Length > 0 && runWords.Count > 0)
            {
                var prevSeg = all[runStart - 1];
                var nextSeg = all[runStart];
                string prevLast = prevSeg.Text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).LastOrDefault() ?? "";
                string nextFirst = nextSeg.Text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).FirstOrDefault() ?? "";
                if (ShouldDehyphenate(prevLast, nextFirst, prevSeg, nextSeg)) { if (text.Length > 0) text.Remove(text.Length - 1, 1); }
                else if (SegmentsNeedSpace(prevSeg, prevLast, nextSeg, nextFirst)) text.Append(' ');
            }

            int spanStart = Utf8Len(text.ToString());
            for (int wi = 0; wi < runWords.Count; wi++)
            {
                if (wi > 0)
                {
                    var (prev, prevIdx) = runWords[wi - 1];
                    var (word, wordIdx) = runWords[wi];
                    if (ShouldDehyphenate(prev, word, all[prevIdx], all[wordIdx])) { if (text.Length > 0) text.Remove(text.Length - 1, 1); }
                    else if (prevIdx == wordIdx) { if (NeedsSpaceBetween(prev, word)) text.Append(' '); }
                    else if (SegmentsNeedSpace(all[prevIdx], prev, all[wordIdx], word)) text.Append(' ');
                }
                text.Append(runWords[wi].Word);
            }
            int spanEnd = Utf8Len(text.ToString());

            if (spanStart < spanEnd)
            {
                if (bold) annotations.Add(new TextAnnotation { Start = (uint)spanStart, End = (uint)spanEnd, Kind = AnnotationKind.Bold });
                if (italic) annotations.Add(new TextAnnotation { Start = (uint)spanStart, End = (uint)spanEnd, Kind = AnnotationKind.Italic });
            }
        }
        return (text.ToString(), annotations);
    }

    private static string JoinLineTextsPlain(List<PdfLine> lines)
    {
        if (lines.Count == 0) return "";
        var wordsPerLine = lines
            .Select(l => l.Segments
                .SelectMany(seg => seg.Text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries)
                    .Select(word => (Word: word, Segment: seg)))
                .ToList())
            .ToList();
        var result = new StringBuilder();
        for (int li = 0; li < wordsPerLine.Count; li++)
        {
            var lineWords = wordsPerLine[li];
            for (int wi = 0; wi < lineWords.Count; wi++)
            {
                var (word, seg) = lineWords[wi];
                if (result.Length == 0) { result.Append(word); continue; }

                (string Word, SegmentData Segment)? prev = null;
                if (wi > 0) prev = lineWords[wi - 1];
                else
                    for (int p = li - 1; p >= 0; p--)
                        if (wordsPerLine[p].Count > 0) { prev = wordsPerLine[p][^1]; break; }
                if (prev is not { } previous) { result.Append(word); continue; }

                if (ShouldDehyphenate(previous.Word, word, previous.Segment, seg))
                {
                    if (result.Length > 0) result.Remove(result.Length - 1, 1);
                    result.Append(word);
                    continue;
                }
                bool insertSpace = ReferenceEquals(previous.Segment, seg)
                    ? NeedsSpaceBetween(previous.Word, word)
                    : SegmentsNeedSpace(previous.Segment, previous.Word, seg, word);
                if (insertSpace) result.Append(' ');
                result.Append(word);
            }
        }
        return result.ToString();
    }

    /// <summary>
    /// Whether a space belongs between the last word of one segment and the first of the next.
    /// </summary>
    /// <remarks>
    /// The producer splits a single word into several spans at kerning-run boundaries
    /// ("elit" becomes "eli" + "t"). Those spans sit flush on one baseline, unlike spans a real
    /// space character separates, so geometry is what tells them apart. Segments on different
    /// lines, in different styles, or with an explicit space at the boundary always take one.
    /// Ports Rust <c>segments_need_space</c>.
    /// </remarks>
    /// <summary>
    /// Maximum horizontal gap between two same-line, same-font-size segments, as a fraction of
    /// the larger font size, that identifies them as one word split across a font-resource
    /// change rather than a genuine inter-word space.
    /// </summary>
    /// <remarks>
    /// Chosen for headroom on both sides rather than to sit at either bound: the reported defect
    /// measures 0.008 em, and a gap the rest of the pipeline treats as a real word boundary sits
    /// at 0.05 em. At 0.05 the outcome would be decided by how <c>fontSize * ratio</c> rounds,
    /// which flips with the font size — and a threshold that flips on font size is not a
    /// threshold. 0.025 em sits about 3x above the defect and 2x below the word-boundary bound
    /// at every font size (xberg-io/xberg#1566).
    /// </remarks>
    private const float TOUCHING_SPAN_GAP_EM_RATIO = 0.025f;

    /// <summary>Maximum baseline difference, in points, for two spans to count as touching.
    /// Tighter than the same-line test, which must tolerate wrapped-line reflow noise.</summary>
    private const float TOUCHING_SPAN_BASELINE_TOLERANCE = 0.05f;

    /// <summary>Maximum font-size difference for two spans to count as touching, as a fraction
    /// of the larger of the two.</summary>
    private const float TOUCHING_SPAN_FONT_SIZE_TOLERANCE_RATIO = 0.01f;

    /// <summary>
    /// Whether two segments are the two halves of one word split across a mid-word font-resource
    /// change: same rotation frame, same baseline, same font size, a gap far below a genuine word
    /// space, and a word character immediately on each side of the boundary.
    /// </summary>
    /// <remarks>
    /// A word drawn as two spans 0.069 pt apart — 0.008 em at 9 pt, against a 2.5 pt space glyph
    /// — came back as "pri js" instead of "prijs". No gap threshold could produce that space,
    /// because the style comparison returns first: a mid-word switch between two embedded subset
    /// fonts whose descriptors disagree reads as a style change with no geometric signal. So this
    /// is checked <em>before</em> any bold/italic/monospace comparison. It can only ever join two
    /// spans, never split them, so it cannot regress a document that already reads correctly
    /// (xberg-io/xberg#1566).
    /// </remarks>
    internal static bool SegmentsAreTouching(
        SegmentData prevSeg, string prevWord, SegmentData nextSeg, string nextWord)
    {
        if (prevWord.Length == 0 || nextWord.Length == 0) return false;
        if (!char.IsLetterOrDigit(prevWord[^1]) || !char.IsLetterOrDigit(nextWord[0])) return false;

        // An explicitly drawn space at the boundary outranks any geometry: the producer emitted a
        // space glyph, and its advance is already inside the previous segment's extent, so the
        // measured gap collapses to ~0 and reads as "touching" — exactly the shape this fires on.
        // Words are whitespace-split, so neither word can reveal it; only the segment text can.
        if ((prevSeg.Text.Length > 0 && char.IsWhiteSpace(prevSeg.Text[^1]))
            || (nextSeg.Text.Length > 0 && char.IsWhiteSpace(nextSeg.Text[0])))
            return false;

        if (!prevSeg.HasSameRotation(nextSeg)) return false;

        if (Math.Abs(prevSeg.UprightBaseline() - nextSeg.UprightBaseline()) > TOUCHING_SPAN_BASELINE_TOLERANCE)
            return false;

        float maxFontSize = Math.Max(Math.Max(prevSeg.FontSize, nextSeg.FontSize), 1.0f);
        if (Math.Abs(prevSeg.FontSize - nextSeg.FontSize) > maxFontSize * TOUCHING_SPAN_FONT_SIZE_TOLERANCE_RATIO)
            return false;

        float gap = nextSeg.UprightAdvanceExtent().Start - prevSeg.UprightAdvanceExtent().End;
        return Math.Abs(gap) < maxFontSize * TOUCHING_SPAN_GAP_EM_RATIO;
    }

    private static bool SegmentsNeedSpace(SegmentData prevSeg, string prevWord, SegmentData nextSeg, string nextWord)
    {
        if (!NeedsSpaceBetween(prevWord, nextWord)) return false;

        if (SegmentsAreTouching(prevSeg, prevWord, nextSeg, nextWord)) return false;

        bool explicitBoundarySpace =
            (prevSeg.Text.Length > 0 && char.IsWhiteSpace(prevSeg.Text[^1]))
            || (nextSeg.Text.Length > 0 && char.IsWhiteSpace(nextSeg.Text[0]));
        if (explicitBoundarySpace) return true;

        if (prevSeg.IsBold != nextSeg.IsBold
            || prevSeg.IsItalic != nextSeg.IsItalic
            || prevSeg.IsMonospace != nextSeg.IsMonospace)
            return true;

        // Runs in different frames are never adjacent glyphs, whatever their page coordinates
        // say, so they always take a separator.
        if (!prevSeg.HasSameRotation(nextSeg)) return true;

        float effHeight = Math.Max(Math.Max(nextSeg.Height, prevSeg.Height), nextSeg.FontSize * 0.5f);
        bool sameLine = Math.Abs(prevSeg.UprightBaseline() - nextSeg.UprightBaseline()) < effHeight * 0.5f;
        if (!sameLine) return true;

        float advanceGap = nextSeg.UprightAdvanceExtent().Start - prevSeg.UprightAdvanceExtent().End;
        return advanceGap > nextSeg.FontSize * SEGMENT_GAP_SPACE_RATIO;
    }

    /// <summary>
    /// Whether a line-ending hyphen should be dropped and the two words joined.
    /// </summary>
    /// <remarks>
    /// The text pattern alone — trailing <c>-</c>, a letter before it, lowercase after — is
    /// also what a <em>suspended</em> hyphen looks like mid-line: Dutch "onderhouds- en" and
    /// "CV- en" matched it and were welded into "onderhoudsen" and "CVen", neither of which is a
    /// word. So the two runs must additionally sit on genuinely different visual lines
    /// (xberg-io/xberg#1581). The check strictly narrows the predicate, so it cannot create a
    /// join anywhere one did not already happen.
    /// </remarks>
    private static bool ShouldDehyphenate(string prev, string next, SegmentData prevSeg, SegmentData nextSeg)
    {
        if (prev.Length < 2 || !prev.EndsWith('-')) return false;
        char? beforeHyphen = prev.Length >= 2 ? prev[^2] : (char?)null;
        if (!(beforeHyphen.HasValue && char.IsLetter(beforeHyphen.Value))) return false;
        if (!(next.Length > 0 && char.IsLower(next[0]))) return false;
        return CrossesVisualLineBreak(prevSeg, nextSeg);
    }

    /// <summary>
    /// Maximum baseline difference, in points, for two runs to count as one visual line rather
    /// than two split by a real line break.
    /// </summary>
    /// <remarks>
    /// Deliberately its own constant rather than a reuse of
    /// <see cref="INLINE_STYLE_BASELINE_TOLERANCE"/> (same value): that one drives the
    /// inline-style grouping call sites, and coupling dehyphenation to it would let an unrelated
    /// change there silently retune this.
    /// </remarks>
    private const float LINE_BREAK_BASELINE_TOLERANCE = 0.5f;

    /// <summary>
    /// Whether two runs sit on genuinely different visual lines, rather than being one line
    /// split by a style-run or font-resource boundary.
    /// </summary>
    private static bool CrossesVisualLineBreak(SegmentData trailing, SegmentData leading)
    {
        if (!trailing.HasSameRotation(leading)) return false;
        float trailingBaseline = trailing.UprightBaseline();
        float leadingBaseline = leading.UprightBaseline();
        return float.IsFinite(trailingBaseline) && float.IsFinite(leadingBaseline)
            && Math.Abs(trailingBaseline - leadingBaseline) > LINE_BREAK_BASELINE_TOLERANCE;
    }

    private static bool NeedsSpaceBetween(string prev, string next)
    {
        bool prevCjk = prev.Length > 0 && IsCjkChar(prev[^1]);
        bool nextCjk = next.Length > 0 && IsCjkChar(next[0]);
        return !(prevCjk && nextCjk);
    }

    private static bool IsCjkChar(char c)
    {
        int cp = c;
        return (cp >= 0x4E00 && cp <= 0x9FFF) || (cp >= 0x3040 && cp <= 0x309F) || (cp >= 0x30A0 && cp <= 0x30FF)
            || (cp >= 0xAC00 && cp <= 0xD7AF) || (cp >= 0x3400 && cp <= 0x4DBF) || (cp >= 0xF900 && cp <= 0xFAFF);
    }

    private static string CollapseInnerSpaces(string line)
    {
        int leading = line.Length - line.TrimStart(' ').Length;
        string prefix = line.Substring(0, leading);
        string rest = line.Substring(leading);
        if (!rest.Contains("  ")) return line;
        var result = new StringBuilder(line.Length);
        result.Append(prefix);
        bool prevSpace = false;
        foreach (char ch in rest)
        {
            if (ch == ' ') { if (!prevSpace) result.Append(ch); prevSpace = true; }
            else { prevSpace = false; result.Append(ch); }
        }
        return result.ToString();
    }

    /// <summary>
    /// Whether the item carries an ordered marker — the same parser <see cref="NormalizeListText"/>
    /// strips with, so an item whose marker was removed is never then rendered as a bullet
    /// (Rust <c>list_item_is_ordered</c>). Lettered, roman, bracketed and parenthesised markers
    /// all count, not just a digit followed by a dot.
    /// </summary>
    private static bool ListItemIsOrdered(PdfParagraph para)
    {
        string text = para.Text.Length > 0 ? para.Text
            : (para.Lines.FirstOrDefault()?.Segments.FirstOrDefault()?.Text ?? "");
        return PdfListMarker.Parse(text) is not null;
    }

    private static readonly char[] DashBullets = { '–', '—', '−', '‐', '‑', '‒', '―', '➤', '►', '▶', '○', '●', '◦' };

    /// <summary>
    /// The item's text with its marker removed, and how many bytes that removal took off the
    /// front — the inline annotations were measured against the un-normalized string, so they
    /// have to move with it (Rust <c>normalize_list_text</c>).
    /// </summary>
    private static (string Normalized, int RemovedPrefixLen) NormalizeListText(string text)
    {
        (string, int) Removed(string normalized) => (normalized, Utf8Len(text) - Utf8Len(normalized));

        if (PdfListMarker.Parse(text) is { } marker && marker.ContentStart <= text.Length)
            return (text[marker.ContentStart..], Utf8Len(text[..marker.ContentStart]));

        string trimmed = text.TrimStart();
        if (trimmed.StartsWith('•')) return Removed(trimmed.Substring(1).TrimStart());
        if (trimmed.StartsWith('·')) return Removed(trimmed.Substring(1).TrimStart());
        if (trimmed.StartsWith("* ")) return Removed(trimmed.Substring(2).TrimStart());
        if (trimmed.StartsWith("- ")) return Removed(trimmed.Substring(2));
        foreach (char ch in DashBullets) if (trimmed.StartsWith(ch)) return Removed(trimmed.Substring(1).TrimStart());
        // Numbered prefix "1. " / "1) "
        int digitEnd = 0;
        while (digitEnd < trimmed.Length && char.IsAsciiDigit(trimmed[digitEnd])) digitEnd++;
        if (digitEnd > 0 && digitEnd < trimmed.Length && (trimmed[digitEnd] == '.' || trimmed[digitEnd] == ')'))
            return Removed(trimmed[(digitEnd + 1)..].TrimStart());
        return Removed(trimmed);
    }

    /// <summary>
    /// Move the annotations left by the marker the normalization took off, clamp them into the
    /// shortened string and drop the ones the removal swallowed whole (Rust
    /// <c>shift_annotations_after_prefix_removal</c>).
    /// </summary>
    private static List<TextAnnotation> ShiftAnnotationsAfterPrefixRemoval(
        List<TextAnnotation> annotations, int removedPrefixLen, int normalizedLen)
    {
        uint removed = (uint)Math.Max(removedPrefixLen, 0);
        uint limit = (uint)Math.Max(normalizedLen, 0);
        var shifted = new List<TextAnnotation>(annotations.Count);
        foreach (var annotation in annotations)
        {
            uint start = Math.Min(annotation.Start >= removed ? annotation.Start - removed : 0u, limit);
            uint end = Math.Min(annotation.End >= removed ? annotation.End - removed : 0u, limit);
            if (start < end) shifted.Add(new TextAnnotation { Start = start, End = end, Kind = annotation.Kind });
        }
        return shifted;
    }

    private static int Utf8Len(string s) => Encoding.UTF8.GetByteCount(s);

    // ── text repair (text_repair.rs) ─────────────────────────────────────────────

    private static string RepairContextualLigatures(string text)
    {
        if (text.Length < 2) return text;
        var bytes = Encoding.UTF8.GetBytes(text);
        var result = new StringBuilder(text.Length + 16);
        bool repaired = false;
        bool prevIsAlpha = false;
        bool prevIsSpaceOrStart = true;
        int byteIdx = 0;

        // Iterate by Unicode scalar (mirrors Rust `chars()`); next-char tests read the raw
        // next UTF-8 byte cast to char, so they are only true for ASCII letters.
        foreach (var rune in text.EnumerateRunes())
        {
            string chStr = rune.ToString();
            int charLen = Encoding.UTF8.GetByteCount(chStr);
            int nextByteIdx = byteIdx + charLen;
            char ch = chStr[0];

            bool nextIsAlpha = nextByteIdx < bytes.Length && char.IsLetter((char)bytes[nextByteIdx]);
            bool nextIsLower = nextByteIdx < bytes.Length && char.IsLower((char)bytes[nextByteIdx]);
            bool nextIsVowel = nextByteIdx < bytes.Length && "aeiouAEIOU".IndexOf((char)bytes[nextByteIdx]) >= 0;

            switch (ch)
            {
                case '!' when prevIsAlpha && nextIsVowel: result.Append("ff"); repaired = true; break;
                case '!' when prevIsAlpha && nextIsAlpha: result.Append("fi"); repaired = true; break;
                case '"' when prevIsAlpha && nextIsAlpha: result.Append("ffi"); repaired = true; break;
                case '#' when prevIsAlpha && nextIsAlpha: result.Append("fi"); repaired = true; break;
                case '#' when prevIsSpaceOrStart && nextIsLower: result.Append("fi"); repaired = true; break;
                case '!' when prevIsSpaceOrStart && nextIsLower: result.Append("fi"); repaired = true; break;
                case '*' when prevIsAlpha && nextIsAlpha: result.Append("tt"); repaired = true; break;
                case ':' when prevIsAlpha && nextIsLower: result.Append("ti"); repaired = true; break;
                case 'M' when prevIsAlpha && !prevIsSpaceOrStart:
                    bool prevWasLower = byteIdx > 0 && char.IsLower((char)bytes[byteIdx - 1]);
                    if (prevWasLower && nextIsLower) { result.Append("tti"); repaired = true; }
                    else result.Append(ch);
                    break;
                default: result.Append(chStr); break;
            }

            prevIsAlpha = char.IsLetter(chStr, 0);
            prevIsSpaceOrStart = chStr.Length == 1 && char.IsWhiteSpace(ch);
            byteIdx = nextByteIdx;
        }
        return repaired ? result.ToString() : text;
    }

    private static string ExpandLigaturesWithSpaceAbsorption(string text)
    {
        if (text.IndexOfAny(new[] { 'ﬀ', 'ﬁ', 'ﬂ', 'ﬃ', 'ﬄ', 'ﬅ', 'ﬆ' }) < 0)
            return text;
        var result = new StringBuilder(text.Length);
        int i = 0;
        while (i < text.Length)
        {
            char ch = text[i];
            string? expansion = ch switch
            {
                'ﬀ' => "ff", 'ﬁ' => "fi", 'ﬂ' => "fl",
                'ﬃ' => "ffi", 'ﬄ' => "ffl", 'ﬅ' => "st", 'ﬆ' => "st",
                _ => null,
            };
            if (expansion is null) { result.Append(ch); i++; continue; }
            result.Append(expansion);
            i++;
            if (i < text.Length && text[i] == ' ')
            {
                if (i + 1 < text.Length && (char.IsLetterOrDigit(text[i + 1]) || text[i + 1] == '_')) i++; // absorb space
            }
        }
        return result.ToString();
    }

    private static string NormalizeUnicodeText(string text)
    {
        if (text.IndexOfAny(new[] { '‘', '’', '“', '”', '⁄', '•' }) < 0)
            return text;
        return text
            .Replace('‘', '\'').Replace('’', '\'')
            .Replace('“', '"').Replace('”', '"')
            .Replace('⁄', '/')
            .Replace('•', '·');
    }

    private static string FinalizeHyphens(string text)
    {
        string collapsed = CollapseSpacedHyphens(text);
        if (collapsed.IndexOfAny(new[] { '‐', '‑' }) < 0) return collapsed;
        return collapsed.Replace('‐', '-').Replace('‑', '-');
    }

    private static string CollapseSpacedHyphens(string text)
    {
        if (text.IndexOfAny(new[] { '‐', '‑' }) < 0) return text;
        bool IsGap(char c) => c is ' ' or ' ' or '\n' or '\r' or '\t';
        var chars = text.ToCharArray();
        var result = new StringBuilder(text.Length);
        int i = 0;
        while (i < chars.Length)
        {
            if (char.IsLetterOrDigit(chars[i]))
            {
                int j = i + 1;
                while (j < chars.Length && IsGap(chars[j])) j++;
                if (j > i + 1 && j < chars.Length && (chars[j] is '‐' or '‑'))
                {
                    int k = j + 1;
                    while (k < chars.Length && IsGap(chars[k])) k++;
                    if (k > j + 1 && k < chars.Length && char.IsLetterOrDigit(chars[k]))
                    {
                        result.Append(chars[i]);
                        result.Append('-');
                        i = k;
                        continue;
                    }
                }
            }
            result.Append(chars[i]);
            i++;
        }
        return result.ToString();
    }
}
